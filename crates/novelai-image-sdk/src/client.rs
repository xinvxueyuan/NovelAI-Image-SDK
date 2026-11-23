//! The NovelAI HTTP client.
//!
//! Port of `novelai_image_mcp.nai.client.NovelAIClient`.
//!
//! Every request carries the Chrome fingerprint header block plus fresh
//! tracking headers and (except login) a bearer token. Responses are decoded by
//! sniffing ZIP vs length-prefixed MessagePack, because NovelAI has been
//! observed returning both shapes from the same endpoint.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::Engine as _;
use lru::LruCache;
use serde_json::{Map, Value};
use sha2::Digest;
use tokio::sync::OnceCell;

use crate::auth::{TrackingHeaders, access_key};
use crate::config::ClientConfig;
use crate::constants::{Endpoint, Model};
use crate::error::Error;
use crate::models::GenerationRequest;
use crate::payload::build_generation_payload;
use crate::response::{
    GenerationEvent, NovelAIImage, check_status, is_zip, parse_messagepack_images, parse_zip_images,
};

/// A stream of generation events.
pub type GenerationStream =
    futures_util::stream::BoxStream<'static, Result<GenerationEvent, Error>>;

/// The NovelAI API client. Cheap to clone; the underlying HTTP session and
/// caches are shared.
#[derive(Clone)]
pub struct NovelAiClient {
    inner: Arc<Inner>,
}

struct Inner {
    config: ClientConfig,
    http: wreq::Client,
    access_token: OnceCell<String>,
    vibe_cache: Mutex<LruCache<String, String>>,
}

impl std::fmt::Debug for NovelAiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NovelAiClient")
            .field("image_base_url", &self.inner.config.image_base_url)
            .field("account_base_url", &self.inner.config.account_base_url)
            .finish_non_exhaustive()
    }
}

impl NovelAiClient {
    /// Build a client from configuration, creating the fingerprinted HTTP
    /// session.
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        config.validate()?;
        let http = config.profile.build_client()?;
        Self::with_http_client(config, http)
    }

    /// Build a client around an injected HTTP session.
    ///
    /// The session must be built from [`crate::browser::BrowserProfile`] to
    /// clear Cloudflare; this constructor exists for tests and advanced users.
    pub fn with_http_client(config: ClientConfig, http: wreq::Client) -> Result<Self, Error> {
        config.validate()?;
        let capacity = std::num::NonZeroUsize::new(config.vibe_cache_entries)
            .ok_or_else(|| Error::config("vibe_cache_entries must be positive"))?;
        let access_token = OnceCell::new();
        if let Some(token) = config.credentials.token_value() {
            let _ = access_token.set(token.to_owned());
        }
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                http,
                access_token,
                vibe_cache: Mutex::new(LruCache::new(capacity)),
            }),
        })
    }

    /// The active configuration (credentials omitted from `Debug`).
    pub fn config(&self) -> &ClientConfig {
        &self.inner.config
    }
    // ── auth ───────────────────────────────────────────────────────────────

    /// The bearer token, logging in with derived credentials if needed.
    ///
    /// The login happens at most once; failures leave the cell empty so a
    /// later call can retry (the Python client had a race here).
    pub async fn access_token(&self) -> Result<String, Error> {
        self.inner
            .access_token
            .get_or_try_init(|| async {
                let credentials = &self.inner.config.credentials;
                let key = match credentials.token_value() {
                    Some(token) => return Ok(token.to_owned()),
                    None => access_key(credentials)?,
                };
                let mut body = Map::new();
                body.insert("key".into(), Value::from(key));
                let content = self
                    .send(
                        wreq::Method::POST,
                        &format!(
                            "{}{}",
                            self.inner.config.account_base_url,
                            Endpoint::Login.as_str()
                        ),
                        None,
                        Some(Value::Object(body)),
                        None,
                        None,
                    )
                    .await?;
                let value = json_object(&content)?;
                match value.get("accessToken").and_then(Value::as_str) {
                    Some(token) if !token.trim().is_empty() => Ok(token.to_owned()),
                    _ => Err(Error::response(
                        "NovelAI login did not return an access token",
                    )),
                }
            })
            .await
            .cloned()
    }

    /// Derive the 64-character login key for the configured credentials.
    pub fn derive_login_key(&self) -> Result<String, Error> {
        access_key(&self.inner.config.credentials)
    }

    // ── generation ────────────────────────────────────────────────────────

    /// Generate images. V3/Furry models hit the ZIP endpoint, V4+ the
    /// MessagePack stream endpoint; the response shape is sniffed either way.
    pub async fn generate(&self, request: &GenerationRequest) -> Result<Vec<NovelAIImage>, Error> {
        let request = request.clone().normalize()?;
        let prepared = self.prepare_references(request).await?;
        let endpoint = if prepared.model.is_v4() {
            Endpoint::ImageStream
        } else {
            Endpoint::Image
        };
        let content = self
            .request(
                wreq::Method::POST,
                &format!("{}{}", self.inner.config.image_base_url, endpoint.as_str()),
                true,
                Some(build_generation_payload(&prepared)),
                None,
                None,
            )
            .await?;
        decode_images(&content)
    }

    /// Encode a vibe-transfer reference image into a reusable token.
    ///
    /// Results are cached (LRU) keyed by host, reference, information and
    /// model, so repeated generations reuse one encode call.
    pub async fn encode_vibe(
        &self,
        reference: &str,
        information_extracted: f64,
        model: Model,
    ) -> Result<String, Error> {
        let key = {
            let digest = sha2::Sha256::digest(
                format!(
                    "{}\0{}\0{}\0{}",
                    self.inner.config.image_base_url,
                    reference,
                    information_extracted,
                    model.as_wire()
                )
                .as_bytes(),
            );
            hex(&digest)
        };
        // Scope the guard so it is provably released before the network await.
        if let Some(cached) = {
            let mut cache = self
                .inner
                .vibe_cache
                .lock()
                .map_err(|_| Error::config("vibe cache lock poisoned"))?;
            cache.get(&key).cloned()
        } {
            return Ok(cached);
        }

        let mut body = Map::new();
        body.insert("image".into(), Value::from(reference));
        body.insert(
            "information_extracted".into(),
            Value::from(information_extracted),
        );
        body.insert("model".into(), Value::from(model.as_wire()));
        let content = self
            .request(
                wreq::Method::POST,
                &format!(
                    "{}{}",
                    self.inner.config.image_base_url,
                    Endpoint::EncodeVibe.as_str()
                ),
                true,
                Some(Value::Object(body)),
                None,
                None,
            )
            .await?;
        let token = base64::engine::general_purpose::STANDARD.encode(&content);
        {
            let mut cache = self
                .inner
                .vibe_cache
                .lock()
                .map_err(|_| Error::config("vibe cache lock poisoned"))?;
            cache.put(key, token.clone());
        }
        Ok(token)
    }

    async fn prepare_references(
        &self,
        mut request: GenerationRequest,
    ) -> Result<GenerationRequest, Error> {
        if !request.model.is_v4() || request.references.is_empty() {
            return Ok(request);
        }
        if request.model.is_v5() {
            return Err(Error::validation(
                "vibe transfer is not supported on V5 models yet",
            ));
        }
        let information = if request.reference_information.is_empty() {
            vec![1.0; request.references.len()]
        } else {
            request.reference_information.clone()
        };
        let mut encoded = Vec::with_capacity(request.references.len());
        for (index, reference) in request.references.iter().enumerate() {
            encoded.push(
                self.encode_vibe(reference, information[index], request.model)
                    .await?,
            );
        }
        request.references = encoded;
        request.reference_information.clear();
        Ok(request)
    }

    // ── transport ────────────────────────────────────────────────────────

    /// Build a request with the Chrome header block, tracking headers and
    /// optional bearer token.
    fn base_request(
        &self,
        method: wreq::Method,
        url: &str,
        token: Option<&str>,
    ) -> wreq::RequestBuilder {
        // Every non-streaming request advertises `*/*`, matching the
        // reference client's browser header block.
        self.base_request_with(method, url, token, &[("Accept", "*/*")])
    }

    /// Like [`Self::base_request`], but lets the caller override headers after
    /// the profile block (e.g. `Accept: application/x-msgpack`).
    ///
    /// Header order is load-bearing: `wreq` keeps duplicate values for
    /// singleton headers, so a request-level `Accept` after the profile block
    /// is what the server sees first. Overrides therefore go last.
    fn base_request_with(
        &self,
        method: wreq::Method,
        url: &str,
        token: Option<&str>,
        overrides: &[(&str, &str)],
    ) -> wreq::RequestBuilder {
        let profile = &self.inner.config.profile;
        let mut builder = profile
            .apply(self.inner.http.request(method, url))
            // `wreq` merges client default headers after request headers,
            // which would re-add the emulation's own `Accept`/UA block.
            .default_headers(false);
        builder = profile.apply_headers(builder);
        builder = builder.header("Content-Type", "application/json");
        for (name, value) in TrackingHeaders::generate().iter() {
            builder = builder.header(name, value);
        }
        if let Some(token) = token {
            builder = builder.header("Authorization", format!("Bearer {token}"));
        }
        for (name, value) in overrides {
            builder = builder.header(*name, *value);
        }
        builder
    }

    async fn request(
        &self,
        method: wreq::Method,
        url: &str,
        authenticated: bool,
        json_body: Option<Value>,
        params: Option<&[(&str, &str)]>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, Error> {
        let token = if authenticated {
            Some(self.access_token().await?)
        } else {
            None
        };
        self.send(method, url, token.as_deref(), json_body, params, timeout)
            .await
    }

    /// The I/O core: no token resolution, so it is safe to call from inside
    /// the login path.
    async fn send(
        &self,
        method: wreq::Method,
        url: &str,
        token: Option<&str>,
        json_body: Option<Value>,
        params: Option<&[(&str, &str)]>,
        timeout: Option<Duration>,
    ) -> Result<Vec<u8>, Error> {
        let url = with_query(url, params);
        let mut builder = self.base_request(method, &url, token);
        if let Some(body) = json_body {
            builder = builder.json(&body);
        }
        let timeout = timeout.unwrap_or(self.inner.config.timeout);

        let response = tokio::time::timeout(timeout, builder.send())
            .await
            .map_err(|_| Error::Timeout {
                message: "NovelAI request timed out".into(),
            })?
            .map_err(transport_error)?;
        let status = response.status().as_u16();
        let content = response.bytes().await.map_err(transport_error)?;
        check_status(status, &content)?;
        Ok(content.to_vec())
    }
}

mod stream;
mod tools;

// ── helpers ─────────────────────────────────────────────────────────────────

/// Read a whole response body (used for error responses and streams).
pub(super) async fn drain_body(
    body: &mut (impl futures_util::Stream<Item = Result<bytes::Bytes, wreq::Error>> + Unpin),
) -> Result<Vec<u8>, Error> {
    use futures_util::StreamExt;
    let mut collected = Vec::new();
    while let Some(chunk) = body.next().await {
        collected.extend_from_slice(&chunk.map_err(transport_error)?);
    }
    Ok(collected)
}

fn transport_error(error: wreq::Error) -> Error {
    let message = if error.is_timeout() {
        "NovelAI request timed out".to_owned()
    } else {
        "NovelAI request transport failed".to_owned()
    };
    if error.is_timeout() {
        Error::Timeout { message }
    } else {
        Error::Transport { message }
    }
}

fn json_object(content: &[u8]) -> Result<Value, Error> {
    let value: Value = serde_json::from_slice(content)
        .map_err(|_| Error::response("NovelAI returned invalid JSON"))?;
    if !value.is_object() {
        return Err(Error::response(
            "NovelAI returned a non-object JSON response",
        ));
    }
    Ok(value)
}

fn first_image(content: Vec<u8>, filename: &str) -> Result<NovelAIImage, Error> {
    if is_zip(&content) {
        let mut images = parse_zip_images(&content)?;
        if images.is_empty() {
            return Err(Error::response("NovelAI returned an empty image archive"));
        }
        let mut image = images.remove(0);
        image.filename = filename.to_owned();
        return Ok(image);
    }
    Ok(NovelAIImage {
        filename: filename.to_owned(),
        data: content,
    })
}

fn decode_images(content: &[u8]) -> Result<Vec<NovelAIImage>, Error> {
    if content.starts_with(b"PK\x03\x04") {
        parse_zip_images(content)
    } else {
        parse_messagepack_images(content)
    }
}

fn with_query(url: &str, params: Option<&[(&str, &str)]>) -> String {
    match params {
        None => url.to_owned(),
        Some(params) => {
            let encoded = params
                .iter()
                .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
                .collect::<Vec<_>>()
                .join("&");
            format!(
                "{url}{}{encoded}",
                if url.contains('?') { '&' } else { '?' }
            )
        }
    }
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".to_owned(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut out, b| {
            let _ = write!(out, "{b:02x}");
            out
        })
}

/// Headers sent with every request (for documentation and host integrations).
pub fn default_headers() -> HashMap<String, String> {
    let mut map = HashMap::new();
    for (name, value) in crate::browser::browser_headers() {
        map.insert(name.to_owned(), value.to_owned());
    }
    map.insert("Content-Type".into(), "application/json".into());
    map
}

#[cfg(test)]
mod tests;
