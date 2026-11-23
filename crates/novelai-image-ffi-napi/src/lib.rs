//! napi-rs boundary for `novelai-image-sdk` (Node.js `>= 20.17`).
//!
//! Async core methods map to JavaScript Promises; binary payloads map to
//! `Buffer`. Streaming is exposed as a pull-based `GenerationStream` object
//! (`await next()`), because callback-style streams do not fit Node's
//! promise model cleanly.

#![allow(missing_docs)]

use std::sync::Arc;

use napi::bindgen_prelude::{Buffer, Result};
use napi_derive::napi;

use novelai_image_sdk::auth::Credentials as CoreCredentials;
use novelai_image_sdk::client::NovelAiClient as CoreClient;
use novelai_image_sdk::config::ClientConfig as CoreConfig;
use novelai_image_sdk::constants as core;

mod types;
pub use types::*;

fn to_js_error(error: novelai_image_sdk::error::Error) -> napi::Error {
    napi::Error::from_reason(error.to_string())
}

/// An access token or a username/password pair.
#[napi(object)]
pub struct Credentials {
    /// A pre-issued NovelAI access token.
    pub token: Option<String>,
    /// The account email/username.
    pub username: Option<String>,
    /// The account password (used to derive the login key; never stored).
    pub password: Option<String>,
}

impl Credentials {
    fn to_core(&self) -> Result<CoreCredentials> {
        let credentials = match (&self.token, &self.username, &self.password) {
            (Some(token), _, _) => CoreCredentials::token(token.clone()),
            (None, Some(username), Some(password)) => {
                CoreCredentials::password(username.clone(), password.clone())
            }
            _ => {
                return Err(napi::Error::from_reason(
                    "provide a token or a complete username/password pair",
                ));
            }
        };
        credentials.validate().map_err(to_js_error)?;
        Ok(credentials)
    }
}

/// Client configuration. Omitted fields take the SDK defaults.
#[napi(object)]
pub struct ClientOptions {
    /// Override the image host (default `https://image.novelai.net`).
    pub image_base_url: Option<String>,
    /// Override the account host.
    pub account_base_url: Option<String>,
    /// Override the legacy host for `/ai/annotate-image`.
    pub legacy_image_base_url: Option<String>,
    /// Per-request timeout in seconds (default 120).
    pub timeout_secs: Option<u32>,
    /// Vibe cache capacity (default 64).
    pub vibe_cache_entries: Option<u32>,
}

impl ClientOptions {
    fn to_overrides(&self) -> novelai_image_sdk::config::ConfigOverrides {
        novelai_image_sdk::config::ConfigOverrides {
            image_base_url: self.image_base_url.clone(),
            account_base_url: self.account_base_url.clone(),
            legacy_image_base_url: self.legacy_image_base_url.clone(),
            timeout: self
                .timeout_secs
                .map(|value| std::time::Duration::from_secs(u64::from(value))),
            vibe_cache_entries: self.vibe_cache_entries.map(|value| value as usize),
        }
    }
}

/// A validated request ready to send.
#[napi(object)]
pub struct ResolvedRequest {
    /// Image width after rounding up to a multiple of 64.
    pub width: u32,
    /// Image height after rounding.
    pub height: u32,
    /// The effective img2img strength.
    pub strength: Option<f64>,
    /// The effective img2img noise.
    pub noise: Option<f64>,
    /// The resolved extra noise seed.
    pub extra_noise_seed: Option<u32>,
    /// The prompt including quality tags.
    pub effective_prompt: String,
    /// The negative prompt including the UC preset.
    pub effective_negative_prompt: String,
    /// The condensed JSON payload that will be sent.
    pub payload_json: String,
}

/// The NovelAI client.
#[napi]
pub struct NovelAiClient {
    inner: Arc<CoreClient>,
}

#[napi]
impl NovelAiClient {
    /// Build a client from credentials and optional overrides.
    #[napi(constructor)]
    pub fn new(credentials: Credentials, options: Option<ClientOptions>) -> Result<Self> {
        let credentials = credentials.to_core()?;
        let mut config = CoreConfig::new(credentials);
        if let Some(options) = &options {
            config.apply_overrides(options.to_overrides());
        }
        Ok(Self {
            inner: Arc::new(CoreClient::new(config).map_err(to_js_error)?),
        })
    }

    /// Generate images.
    #[napi]
    pub async fn generate(&self, request: GenerationRequest) -> Result<Vec<Image>> {
        let request = request.to_core()?;
        let images = self.inner.generate(&request).await.map_err(to_js_error)?;
        Ok(images
            .into_iter()
            .map(|image| Image {
                filename: image.filename,
                data: image.data.into(),
            })
            .collect())
    }

    /// Collect a streaming generation into an array of events.
    ///
    /// Node gets the buffered form in v0.1; the Rust core exposes the true
    /// incremental stream, and a callback/`AsyncIterator` surface is planned
    /// once each binding language has a uniform callback story.
    #[napi]
    pub async fn collect_generation(
        &self,
        request: GenerationRequest,
        max_events: Option<u32>,
    ) -> Result<Vec<GenerationEvent>> {
        let request = request.to_core()?;
        let stream = self
            .inner
            .stream_generation(&request)
            .await
            .map_err(to_js_error)?;
        collect_js_events(stream, max_events).await
    }

    /// Validate and resolve a request without sending it.
    #[napi]
    pub fn resolve_request(&self, request: GenerationRequest) -> Result<ResolvedRequest> {
        let request = request.to_core()?;
        let payload = novelai_image_sdk::payload::build_generation_payload(&request);
        Ok(ResolvedRequest {
            width: request.width,
            height: request.height,
            strength: request.strength,
            noise: request.noise,
            extra_noise_seed: request.extra_noise_seed,
            effective_prompt: request.effective_prompt(),
            effective_negative_prompt: request.effective_negative_prompt(),
            payload_json: serde_json::to_string(&payload).unwrap_or_default(),
        })
    }

    /// Estimate the Anlas cost without sending a request.
    #[napi]
    pub fn estimate_anlas_cost(
        &self,
        request: GenerationRequest,
        opus: Option<bool>,
    ) -> Result<u32> {
        let request = request.to_core()?;
        Ok(request.estimate_anlas_cost(opus.unwrap_or(false)))
    }

    /// Encode a vibe reference.
    #[napi]
    pub async fn encode_vibe(
        &self,
        reference: String,
        information_extracted: f64,
        model: Option<String>,
    ) -> Result<String> {
        let model = match model {
            Some(value) => value.parse().map_err(to_js_error)?,
            None => core::Model::V4_5,
        };
        self.inner
            .encode_vibe(&reference, information_extracted, model)
            .await
            .map_err(to_js_error)
    }

    /// Upscale an image (V5 only, 4x).
    #[napi]
    pub async fn upscale(
        &self,
        image: Buffer,
        factor: Option<u32>,
        model: Option<String>,
    ) -> Result<Image> {
        let model = model
            .map(|value| value.parse().map_err(to_js_error))
            .transpose()?;
        let image = self
            .inner
            .upscale(&image, factor.unwrap_or(4), model)
            .await
            .map_err(to_js_error)?;
        Ok(Image {
            filename: image.filename,
            data: image.data.into(),
        })
    }

    /// Annotate an image with a ControlNet model.
    #[napi]
    pub async fn annotate(&self, image: Buffer, model: String) -> Result<Image> {
        let model = model.parse().map_err(to_js_error)?;
        let image = self
            .inner
            .annotate(&image, model)
            .await
            .map_err(to_js_error)?;
        Ok(Image {
            filename: image.filename,
            data: image.data.into(),
        })
    }

    /// Run a Director tool.
    #[napi]
    pub async fn director(
        &self,
        tool: String,
        image: Buffer,
        prompt: Option<String>,
        defry: Option<u32>,
        emotion: Option<String>,
        emotion_level: Option<u8>,
    ) -> Result<Image> {
        let tool: core::DirectorTool = tool.parse().map_err(to_js_error)?;
        let emotion = emotion
            .map(|value| value.parse().map_err(to_js_error))
            .transpose()?;
        let level =
            core::EmotionLevel::try_from(emotion_level.unwrap_or(0)).map_err(to_js_error)?;
        let image = self
            .inner
            .director(
                tool,
                &image,
                &prompt.unwrap_or_default(),
                defry.unwrap_or(0),
                emotion,
                level,
            )
            .await
            .map_err(to_js_error)?;
        Ok(Image {
            filename: image.filename,
            data: image.data.into(),
        })
    }

    /// Ask the tagger for completing tags.
    #[napi]
    pub async fn suggest_tags(
        &self,
        prompt: String,
        model: Option<String>,
        language: Option<String>,
    ) -> Result<Vec<TagSuggestion>> {
        let model = match model {
            Some(value) => value.parse().map_err(to_js_error)?,
            None => core::Model::V4_5,
        };
        let language = language.unwrap_or_else(|| "en".into());
        let tags = self
            .inner
            .suggest_tags(&prompt, model, &language)
            .await
            .map_err(to_js_error)?;
        Ok(tags
            .into_iter()
            .map(|tag| TagSuggestion {
                tag: tag
                    .get("tag")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
                count: tag
                    .get("count")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(0),
                confidence: tag
                    .get("confidence")
                    .and_then(serde_json::Value::as_f64)
                    .unwrap_or(0.0),
            })
            .collect())
    }

    /// The subscription record as JSON.
    #[napi]
    pub async fn get_subscription(&self) -> Result<String> {
        let value = self.inner.get_subscription().await.map_err(to_js_error)?;
        Ok(serde_json::to_string(&value).unwrap_or_default())
    }

    /// The user-data record as JSON.
    #[napi]
    pub async fn get_user_data(&self) -> Result<String> {
        let value = self.inner.get_user_data().await.map_err(to_js_error)?;
        Ok(serde_json::to_string(&value).unwrap_or_default())
    }
}

/// Drain a generation stream into at most `max_events` napi events.
async fn collect_js_events(
    mut stream: novelai_image_sdk::client::GenerationStream,
    max_events: Option<u32>,
) -> Result<Vec<GenerationEvent>> {
    use futures_util::StreamExt;
    let limit = max_events.map_or(usize::MAX, |max| max as usize);
    let mut events = Vec::with_capacity(limit.min(64));
    while events.len() < limit {
        let Some(event) = stream.next().await else {
            break;
        };
        events.push(to_js_event(event.map_err(to_js_error)?));
    }
    Ok(events)
}

/// Convert a core event into its napi representation.
fn to_js_event(event: novelai_image_sdk::response::GenerationEvent) -> GenerationEvent {
    GenerationEvent {
        event_type: event.event_type,
        sample_index: event.sample_index,
        step_index: event.step_index,
        generation_id: event.generation_id,
        sigma: event.sigma,
        image: Image {
            filename: event.image.filename,
            data: event.image.data.into(),
        },
    }
}

/// The SDK version.
#[napi]
pub fn version() -> String {
    novelai_image_sdk::VERSION.to_owned()
}

/// The model catalog.
#[napi]
pub fn list_models() -> Vec<ModelInfo> {
    core::Model::ALL
        .iter()
        .map(|model| ModelInfo {
            id: model.as_wire().to_owned(),
            is_v4: model.is_v4(),
            is_v5: model.is_v5(),
            inpaint: model.is_inpaint(),
            supports_vibe: model.supports_vibe(),
            supports_upscale: model.supports_upscale(),
            params_version: model.params_version(),
            quality_tags: core::quality_tags(*model).to_owned(),
            uc_presets: core::uc_presets(*model)
                .iter()
                .map(|preset| (*preset).to_owned())
                .collect(),
        })
        .collect()
}

/// NovelAI's official explanation for a documented error code.
#[napi]
pub fn error_explanation(code: u16) -> Option<String> {
    novelai_image_sdk::error::explain(Some(&novelai_image_sdk::error::ErrorCode::Http(code)))
        .map(str::to_owned)
}

/// Derive the 64-character NovelAI login key (credential-sensitive; do not log).
#[napi]
pub fn derive_access_key(username: String, password: String) -> Result<String> {
    novelai_image_sdk::auth::derive_access_key(&username, &password).map_err(to_js_error)
}
