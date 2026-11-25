//! UniFFI boundary for `novelai-image-sdk`.
//!
//! Only the types the SDK actually returns cross the boundary; validation and
//! normalization stay in the core crate. `model` crosses as a string (a new
//! NovelAI model must not force re-releasing seven bindings); the stable
//! vocabularies (`Sampler`, `NoiseSchedule`, `ControlNetModel`, `DirectorTool`,
//! `Emotion`, `EmotionLevel`) cross as enums generated from the same
//! definitions the core uses.
//!
//! Build with `cargo build -p novelai-image-ffi-uniffi` and generate bindings
//! with `cargo run -p uniffi-bindgen -- generate --library ...`.

#![allow(missing_docs)]

use std::sync::Arc;

use novelai_image_sdk::client::NovelAiClient as NovelAiClientCore;
use novelai_image_sdk::config::ClientConfig as CoreConfig;
use novelai_image_sdk::constants as core;

uniffi::setup_scaffolding!();

/// A domain failure. `reason` is preformatted by the core crate, including
/// NovelAI's ` [code X: explanation]` suffix.
///
/// The field is called `reason`, not `message`: UniFFI's Kotlin generator
/// emits `val message` for every variant, and that collides with
/// `kotlin.Throwable.message`, so the generated code does not compile.
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[non_exhaustive]
pub enum SdkError {
    /// The request is invalid.
    #[error("{reason}")]
    Validation { reason: String },
    /// Credentials are missing, invalid, or expired.
    #[error("{reason}")]
    Authentication {
        reason: String,
        code: Option<String>,
        explanation: Option<String>,
    },
    /// No credentials were configured.
    #[error("{reason}")]
    MissingToken { reason: String },
    /// The account has insufficient Anlas or no subscription.
    #[error("{reason}")]
    InsufficientCredits { reason: String },
    /// The account hit a concurrency or rate limit.
    #[error("{reason}")]
    Concurrency { reason: String },
    /// NovelAI rejected or failed the request with another status.
    #[error("{reason}")]
    Provider { reason: String },
    /// A request exceeded its timeout.
    #[error("{reason}")]
    Timeout { reason: String },
    /// The request could not reach NovelAI.
    #[error("{reason}")]
    Transport { reason: String },
    /// NovelAI returned malformed data.
    #[error("{reason}")]
    Response { reason: String },
    /// The image is malformed or unsupported.
    #[error("{reason}")]
    Image { reason: String },
    /// The SDK is misconfigured.
    #[error("{reason}")]
    Config { reason: String },
}

impl From<novelai_image_sdk::error::Error> for SdkError {
    fn from(error: novelai_image_sdk::error::Error) -> Self {
        use novelai_image_sdk::error::Error as E;
        let code = |value: &Option<novelai_image_sdk::error::ErrorCode>| {
            value.as_ref().map(ToString::to_string)
        };
        match error {
            E::Validation {
                message,
                code: c,
                explanation,
            } => {
                let _ = (c, explanation);
                Self::Validation { reason: message }
            }
            E::Authentication {
                message,
                code: c,
                explanation,
            } => Self::Authentication {
                reason: message,
                code: code(&c),
                explanation,
            },
            E::MissingToken { message } => Self::MissingToken { reason: message },
            E::InsufficientCredits { message, .. } => Self::InsufficientCredits { reason: message },
            E::Concurrency { message, .. } => Self::Concurrency { reason: message },
            E::Provider { message, .. } => Self::Provider { reason: message },
            E::Timeout { message } => Self::Timeout { reason: message },
            E::Transport { message } => Self::Transport { reason: message },
            E::Response { message } => Self::Response { reason: message },
            E::Image { message } => Self::Image { reason: message },
            E::Config { message } => Self::Config { reason: message },
            // `Error` is #[non_exhaustive]; surface future variants as provider
            // failures rather than panicking at the FFI boundary.
            other => Self::Provider {
                reason: other.to_string(),
            },
        }
    }
}

mod types;
pub use types::*;

/// The NovelAI client. Thread-safe and cheap to clone.
#[derive(uniffi::Object)]
pub struct NovelAiClient {
    inner: Arc<NovelAiClientInner>,
}

struct NovelAiClientInner {
    client: NovelAiClientCore,
}

#[uniffi::export]
impl NovelAiClient {
    /// Build a client and verify the credentials shape.
    #[uniffi::constructor]
    pub fn new(
        credentials: Credentials,
        options: Option<ClientOptions>,
    ) -> Result<Arc<Self>, SdkError> {
        let credentials = credentials.to_core()?;
        let mut config = CoreConfig::new(credentials);
        if let Some(options) = &options {
            config.apply_overrides(options.to_overrides());
        }
        let client = NovelAiClientCore::new(config).map_err(SdkError::from)?;
        Ok(Arc::new(Self {
            inner: Arc::new(NovelAiClientInner { client }),
        }))
    }

    /// Generate images; V3/Furry return ZIP, V4+/V5 return MessagePack, both
    /// auto-sniffed.
    pub async fn generate(&self, request: GenerationRequest) -> Result<Vec<Image>, SdkError> {
        let request = request.to_core()?;
        let images = self
            .inner
            .client
            .generate(&request)
            .await
            .map_err(SdkError::from)?;
        Ok(images
            .into_iter()
            .map(|image| Image {
                filename: image.filename,
                data: image.data,
            })
            .collect())
    }

    /// Collect a streaming generation into its events (V4+ / V5 only).
    ///
    /// The Rust core exposes a true incremental stream, but UniFFI has no
    /// streaming export; like the napi boundary's `collect_generation`, this
    /// drains the stream into a list. `max_events` caps how many events are
    /// lifted across FFI (the rest stay in the dropped stream).
    pub async fn collect_generation(
        &self,
        request: GenerationRequest,
        max_events: Option<u32>,
    ) -> Result<Vec<GenerationEvent>, SdkError> {
        use futures_util::{StreamExt, TryStreamExt};

        let request = request.to_core()?;
        let stream = self
            .inner
            .client
            .stream_generation(&request)
            .await
            .map_err(SdkError::from)?;
        let limit = max_events.map_or(usize::MAX, |max| max as usize);
        stream
            .take(limit)
            .map(|event| event.map(to_ffi_event).map_err(SdkError::from))
            .try_collect()
            .await
    }

    /// Resolve a request without sending it (validation + defaults + payload).
    pub fn resolve_request(&self, request: GenerationRequest) -> Result<ResolvedRequest, SdkError> {
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

    /// Estimate Anlas cost without sending a request.
    pub fn estimate_anlas_cost(
        &self,
        request: GenerationRequest,
        opus: bool,
    ) -> Result<AnlasEstimate, SdkError> {
        let request = request.to_core()?;
        Ok(AnlasEstimate {
            anlas: request.estimate_anlas_cost(opus),
        })
    }

    /// Encode a vibe reference.
    pub async fn encode_vibe(
        &self,
        reference: String,
        information_extracted: f64,
        model: Option<String>,
    ) -> Result<String, SdkError> {
        let model = match model {
            Some(value) => value.parse().map_err(SdkError::from)?,
            None => core::Model::V4_5,
        };
        self.inner
            .client
            .encode_vibe(&reference, information_extracted, model)
            .await
            .map_err(SdkError::from)
    }

    /// Upscale an image (V5 only, 4x).
    pub async fn upscale(
        &self,
        image: Vec<u8>,
        factor: u32,
        model: Option<String>,
    ) -> Result<Image, SdkError> {
        let model = model
            .map(|value| value.parse().map_err(SdkError::from))
            .transpose()?;
        let image = self
            .inner
            .client
            .upscale(&image, factor, model)
            .await
            .map_err(SdkError::from)?;
        Ok(Image {
            filename: image.filename,
            data: image.data,
        })
    }

    /// Annotate an image with a ControlNet model.
    pub async fn annotate(&self, image: Vec<u8>, model: String) -> Result<Image, SdkError> {
        let model = model.parse().map_err(SdkError::from)?;
        let image = self
            .inner
            .client
            .annotate(&image, model)
            .await
            .map_err(SdkError::from)?;
        Ok(Image {
            filename: image.filename,
            data: image.data,
        })
    }

    /// Run a Director tool.
    pub async fn director(
        &self,
        tool: String,
        image: Vec<u8>,
        prompt: String,
        defry: u32,
        emotion: Option<String>,
        emotion_level: u8,
    ) -> Result<Image, SdkError> {
        let tool: core::DirectorTool = tool.parse().map_err(SdkError::from)?;
        let emotion = emotion
            .map(|value| value.parse().map_err(SdkError::from))
            .transpose()?;
        let level = core::EmotionLevel::try_from(emotion_level).map_err(SdkError::from)?;
        let image = self
            .inner
            .client
            .director(tool, &image, &prompt, defry, emotion, level)
            .await
            .map_err(SdkError::from)?;
        Ok(Image {
            filename: image.filename,
            data: image.data,
        })
    }

    /// Ask the tagger for completing tags.
    pub async fn suggest_tags(
        &self,
        prompt: String,
        model: Option<String>,
        language: Option<String>,
    ) -> Result<Vec<TagSuggestion>, SdkError> {
        let model = match model {
            Some(value) => value.parse().map_err(SdkError::from)?,
            None => core::Model::V4_5,
        };
        let language = language.unwrap_or_else(|| "en".into());
        let tags = self
            .inner
            .client
            .suggest_tags(&prompt, model, &language)
            .await
            .map_err(SdkError::from)?;
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
    pub async fn get_subscription(&self) -> Result<String, SdkError> {
        let value = self
            .inner
            .client
            .get_subscription()
            .await
            .map_err(SdkError::from)?;
        Ok(serde_json::to_string(&value).unwrap_or_default())
    }

    /// The user-data record as JSON.
    pub async fn get_user_data(&self) -> Result<String, SdkError> {
        let value = self
            .inner
            .client
            .get_user_data()
            .await
            .map_err(SdkError::from)?;
        Ok(serde_json::to_string(&value).unwrap_or_default())
    }
}

/// The SDK version.
#[uniffi::export]
pub fn version() -> String {
    novelai_image_sdk::VERSION.to_owned()
}

/// The model catalog, mirroring the MCP server's `novelai://models` resource.
#[uniffi::export]
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
#[uniffi::export]
pub fn error_explanation(code: u16) -> Option<String> {
    novelai_image_sdk::error::explain(Some(&novelai_image_sdk::error::ErrorCode::Http(code)))
        .map(str::to_owned)
}

/// Derive the 64-character NovelAI login key (credential-sensitive; do not log).
#[uniffi::export]
pub fn derive_access_key(username: String, password: String) -> Result<String, SdkError> {
    novelai_image_sdk::auth::derive_access_key(&username, &password).map_err(SdkError::from)
}

/// The Chrome header block every request carries.
#[uniffi::export]
pub fn browser_headers() -> std::collections::HashMap<String, String> {
    novelai_image_sdk::browser::browser_headers()
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// Lift a core streaming event across the UniFFI boundary.
fn to_ffi_event(event: novelai_image_sdk::response::GenerationEvent) -> GenerationEvent {
    GenerationEvent {
        event_type: event.event_type,
        sample_index: event.sample_index,
        step_index: event.step_index,
        generation_id: event.generation_id,
        sigma: event.sigma,
        image: Image {
            filename: event.image.filename,
            data: event.image.data,
        },
    }
}

#[cfg(test)]
mod tests;
