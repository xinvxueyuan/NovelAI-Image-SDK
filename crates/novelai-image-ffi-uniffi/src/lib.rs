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

use novelai_image_sdk::auth::Credentials as CoreCredentials;
use novelai_image_sdk::client::NovelAiClient as NovelAiClientCore;
use novelai_image_sdk::config::ClientConfig as CoreConfig;
use novelai_image_sdk::constants as core;

uniffi::setup_scaffolding!();

/// A domain failure. `message` is preformatted by the core crate, including
/// NovelAI's ` [code X: explanation]` suffix.
#[derive(Debug, thiserror::Error, uniffi::Error)]
#[non_exhaustive]
pub enum SdkError {
    /// The request is invalid.
    #[error("{message}")]
    Validation { message: String },
    /// Credentials are missing, invalid, or expired.
    #[error("{message}")]
    Authentication {
        message: String,
        code: Option<String>,
        explanation: Option<String>,
    },
    /// No credentials were configured.
    #[error("{message}")]
    MissingToken { message: String },
    /// The account has insufficient Anlas or no subscription.
    #[error("{message}")]
    InsufficientCredits { message: String },
    /// The account hit a concurrency or rate limit.
    #[error("{message}")]
    Concurrency { message: String },
    /// NovelAI rejected or failed the request with another status.
    #[error("{message}")]
    Provider { message: String },
    /// A request exceeded its timeout.
    #[error("{message}")]
    Timeout { message: String },
    /// The request could not reach NovelAI.
    #[error("{message}")]
    Transport { message: String },
    /// NovelAI returned malformed data.
    #[error("{message}")]
    Response { message: String },
    /// The image is malformed or unsupported.
    #[error("{message}")]
    Image { message: String },
    /// The SDK is misconfigured.
    #[error("{message}")]
    Config { message: String },
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
                Self::Validation { message }
            }
            E::Authentication {
                message,
                code: c,
                explanation,
            } => Self::Authentication {
                message,
                code: code(&c),
                explanation,
            },
            E::MissingToken { message } => Self::MissingToken { message },
            E::InsufficientCredits { message, .. } => Self::InsufficientCredits { message },
            E::Concurrency { message, .. } => Self::Concurrency { message },
            E::Provider { message, .. } => Self::Provider { message },
            E::Timeout { message } => Self::Timeout { message },
            E::Transport { message } => Self::Transport { message },
            E::Response { message } => Self::Response { message },
            E::Image { message } => Self::Image { message },
            E::Config { message } => Self::Config { message },
            // `Error` is #[non_exhaustive]; surface future variants as provider
            // failures rather than panicking at the FFI boundary.
            other => Self::Provider {
                message: other.to_string(),
            },
        }
    }
}

/// An access token or a username/password pair.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Credentials {
    /// A pre-issued NovelAI access token.
    pub token: Option<String>,
    /// The account email/username.
    pub username: Option<String>,
    /// The account password (used to derive the login key; never stored).
    pub password: Option<String>,
}

impl Credentials {
    fn to_core(&self) -> Result<CoreCredentials, SdkError> {
        let credentials = match (&self.token, &self.username, &self.password) {
            (Some(token), _, _) => CoreCredentials::token(token.clone()),
            (None, Some(username), Some(password)) => {
                CoreCredentials::password(username.clone(), password.clone())
            }
            _ => {
                return Err(SdkError::Validation {
                    message: "provide a token or a complete username/password pair".into(),
                });
            }
        };
        credentials.validate().map_err(SdkError::from)?;
        Ok(credentials)
    }
}

/// Client configuration. Empty fields take the SDK defaults.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct ClientOptions {
    /// Override the image host (default `https://image.novelai.net`).
    pub image_base_url: Option<String>,
    /// Override the account host.
    pub account_base_url: Option<String>,
    /// Override the legacy host for `/ai/annotate-image`.
    pub legacy_image_base_url: Option<String>,
    /// Per-request timeout in seconds (default 120).
    pub timeout_secs: Option<u64>,
    /// Vibe cache capacity (default 64).
    pub vibe_cache_entries: Option<u32>,
}

/// An image returned by NovelAI.
#[derive(Debug, Clone, uniffi::Record)]
pub struct Image {
    /// Suggested filename.
    pub filename: String,
    /// Raw PNG or JPEG bytes.
    pub data: Vec<u8>,
}

/// A streaming-generation event.
#[derive(Debug, Clone, uniffi::Record)]
pub struct GenerationEvent {
    /// `intermediate` or `final`.
    pub event_type: String,
    /// Sample index.
    pub sample_index: i64,
    /// Sampling step.
    pub step_index: i64,
    /// Generation id.
    pub generation_id: String,
    /// Noise sigma.
    pub sigma: f64,
    /// The image at this step.
    pub image: Image,
}

/// One tag suggested by the tagger.
#[derive(Debug, Clone, uniffi::Record)]
pub struct TagSuggestion {
    /// The tag text.
    pub tag: String,
    /// Co-occurrence count.
    pub count: i64,
    /// Confidence in `0.0..=1.0`.
    pub confidence: f64,
}

/// Model capability information (mirrors `novelai://models`).
#[derive(Debug, Clone, uniffi::Record)]
pub struct ModelInfo {
    /// The wire id (`nai-diffusion-4-5-full`).
    pub id: String,
    /// Structured-prompt (V4+) model.
    pub is_v4: bool,
    /// V5 family.
    pub is_v5: bool,
    /// Accepts an inpainting mask.
    pub inpaint: bool,
    /// Supports vibe transfer.
    pub supports_vibe: bool,
    /// Supports standalone upscaling.
    pub supports_upscale: bool,
    /// NovelAI `params_version`.
    pub params_version: u8,
    /// Quality tags appended to the prompt.
    pub quality_tags: String,
    /// The four undesired-content presets.
    pub uc_presets: Vec<String>,
}

/// A generation request. Every field is optional; the core builder applies
/// the documented defaults, then the same validation the Python reference
/// performs.
#[derive(Debug, Clone, Default, uniffi::Record)]
pub struct GenerationRequest {
    /// The positive prompt (required).
    pub prompt: String,
    /// Base caption for structured-prompt models.
    pub base_caption: Option<String>,
    /// Model id; defaults to `nai-diffusion-4-5-full`.
    pub model: Option<String>,
    /// `generate`, `img2img`, or `infill`.
    pub action: Option<String>,
    /// Undesired content.
    pub negative_prompt: Option<String>,
    /// Width in pixels (rounded up to a multiple of 64).
    pub width: Option<u32>,
    /// Height in pixels.
    pub height: Option<u32>,
    /// Number of samples.
    pub n_samples: Option<u32>,
    /// Sampling steps.
    pub steps: Option<u32>,
    /// Guidance scale.
    pub scale: Option<f64>,
    /// Sampler wire id.
    pub sampler: Option<String>,
    /// Seed (`0` = provider-random).
    pub seed: Option<u32>,
    /// img2img extra noise seed (random when absent).
    pub extra_noise_seed: Option<u32>,
    /// Noise schedule wire id.
    pub noise_schedule: Option<String>,
    /// Append quality tags.
    pub quality: Option<bool>,
    /// Undesired-content preset index (`0..=3`).
    pub uc_preset: Option<u8>,
    /// CFG rescale (`0.0..=1.0`).
    pub cfg_rescale: Option<f64>,
    /// Source image, base64.
    pub image: Option<String>,
    /// Inpainting mask, base64.
    pub mask: Option<String>,
    /// img2img denoise strength.
    pub strength: Option<f64>,
    /// img2img noise.
    pub noise: Option<f64>,
    /// Vibe reference tokens.
    pub references: Vec<String>,
    /// Per-reference information-extracted values.
    pub reference_information: Vec<f64>,
    /// Estimate cost only (relaxes the image/mask requirements).
    pub estimate_only: Option<bool>,
}

impl GenerationRequest {
    fn to_core(&self) -> Result<novelai_image_sdk::models::GenerationRequest, SdkError> {
        use novelai_image_sdk::models::GenerationRequest as Core;
        let mut builder = Core::new(self.prompt.clone());
        if let Some(value) = &self.base_caption {
            builder = builder.base_caption(Some(value.clone()));
        }
        if let Some(value) = &self.model {
            builder = builder.model(value.parse().map_err(SdkError::from)?);
        }
        if let Some(value) = &self.action {
            builder = builder.action(value.parse().map_err(SdkError::from)?);
        }
        if let Some(value) = &self.negative_prompt {
            builder = builder.negative_prompt(value.clone());
        }
        if let Some(value) = self.width {
            builder = builder.width(value);
        }
        if let Some(value) = self.height {
            builder = builder.height(value);
        }
        if let Some(value) = self.n_samples {
            builder = builder.n_samples(value);
        }
        if let Some(value) = self.steps {
            builder = builder.steps(value);
        }
        if let Some(value) = self.scale {
            builder = builder.scale(value);
        }
        if let Some(value) = &self.sampler {
            builder = builder.sampler(value.parse().map_err(SdkError::from)?);
        }
        if let Some(value) = self.seed {
            builder = builder.seed(value);
        }
        if let Some(value) = self.extra_noise_seed {
            builder = builder.extra_noise_seed(Some(value));
        }
        if let Some(value) = &self.noise_schedule {
            builder = builder.noise_schedule(value.parse().map_err(SdkError::from)?);
        }
        if let Some(value) = self.quality {
            builder = builder.quality(value);
        }
        if let Some(value) = self.uc_preset {
            builder = builder.uc_preset(value);
        }
        if let Some(value) = self.cfg_rescale {
            builder = builder.cfg_rescale(value);
        }
        builder = builder
            .image(self.image.clone())
            .mask(self.mask.clone())
            .strength(self.strength)
            .noise(self.noise)
            .references(self.references.clone())
            .reference_information(self.reference_information.clone());
        if let Some(value) = self.estimate_only {
            builder = builder.estimate_only(value);
        }
        builder.build().map_err(SdkError::from)
    }
}

/// A settled, validated request — what the client will actually send.
///
/// The random `extra_noise_seed` is filled in here and returned, so callers can
/// log or replay it.
#[derive(Debug, Clone, uniffi::Record)]
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

/// An Anlas price estimate.
#[derive(Debug, Clone, uniffi::Record)]
pub struct AnlasEstimate {
    /// The estimated cost in Anlas.
    pub anlas: u32,
}

/// The NovelAI client. Thread-safe and cheap to clone.
#[derive(uniffi::Object)]
pub struct NovelAiClient {
    inner: Arc<NovelAiClientInner>,
}

struct NovelAiClientInner {
    client: NovelAiClientCore,
}

// Keep the core type name readable while avoiding a clash with the exported name.

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
        if let Some(options) = options {
            if let Some(value) = options.image_base_url {
                config = config.with_image_base_url(value);
            }
            if let Some(value) = options.account_base_url {
                config = config.with_account_base_url(value);
            }
            if let Some(value) = options.legacy_image_base_url {
                config.legacy_image_base_url = value;
            }
            if let Some(value) = options.timeout_secs {
                config.timeout = std::time::Duration::from_secs(value);
            }
            if let Some(value) = options.vibe_cache_entries {
                config.vibe_cache_entries = value as usize;
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_and_functions_expose_the_catalog() {
        let models = list_models();
        assert_eq!(models.len(), 15);
        assert!(
            models
                .iter()
                .any(|m| m.id == "nai-diffusion-5-full" && m.is_v5)
        );
        assert_eq!(
            error_explanation(402).unwrap(),
            "active subscription or Anlas required"
        );
        assert_eq!(browser_headers().len(), 13);
        assert_eq!(version(), novelai_image_sdk::VERSION);
    }

    #[test]
    fn request_resolution_matches_the_core() {
        let client = NovelAiClient::new(
            Credentials {
                token: Some("t".into()),
                username: None,
                password: None,
            },
            None,
        )
        .expect("client");
        let resolved = client
            .resolve_request(GenerationRequest {
                prompt: "1girl".into(),
                width: Some(100),
                ..Default::default()
            })
            .expect("resolve");
        assert_eq!(resolved.width, 128);
        assert!(resolved.effective_prompt.starts_with("1girl"));
        let payload: serde_json::Value = serde_json::from_str(&resolved.payload_json).unwrap();
        assert_eq!(payload["model"], "nai-diffusion-4-5-full");
    }

    #[test]
    fn errors_keep_preformatted_messages() {
        let err = SdkError::from(novelai_image_sdk::error::Error::Authentication {
            message: "NovelAI HTTP 401: Unauthorized".into(),
            code: Some(novelai_image_sdk::error::ErrorCode::Http(401)),
            explanation: Some("authentication failed".into()),
        });
        assert!(err.to_string().contains("NovelAI HTTP 401"));
    }
}
