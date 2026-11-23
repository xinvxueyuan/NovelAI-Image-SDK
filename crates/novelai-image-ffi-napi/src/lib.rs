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

/// An image returned by NovelAI.
#[napi(object)]
pub struct Image {
    /// Suggested filename.
    pub filename: String,
    /// Raw PNG or JPEG bytes.
    pub data: Buffer,
}

/// A streaming-generation event.
#[napi(object)]
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
#[napi(object)]
pub struct TagSuggestion {
    /// The tag text.
    pub tag: String,
    /// Co-occurrence count.
    pub count: i64,
    /// Confidence in `0.0..=1.0`.
    pub confidence: f64,
}

/// Model capability information.
#[napi(object)]
pub struct ModelInfo {
    /// The wire id.
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
/// defaults and the reference validation.
#[napi(object)]
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
    /// img2img extra noise seed.
    pub extra_noise_seed: Option<u32>,
    /// Noise schedule wire id.
    pub noise_schedule: Option<String>,
    /// Append quality tags.
    pub quality: Option<bool>,
    /// Undesired-content preset index (`0..=3`).
    pub uc_preset: Option<u8>,
    /// CFG rescale.
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
    pub references: Option<Vec<String>>,
    /// Estimate cost only.
    pub estimate_only: Option<bool>,
}

impl GenerationRequest {
    fn to_core(&self) -> Result<novelai_image_sdk::models::GenerationRequest> {
        use novelai_image_sdk::models::GenerationRequest as Core;
        let mut builder = Core::new(self.prompt.clone());
        if let Some(value) = &self.base_caption {
            builder = builder.base_caption(Some(value.clone()));
        }
        if let Some(value) = &self.model {
            builder = builder.model(value.parse().map_err(to_js_error)?);
        }
        if let Some(value) = &self.action {
            builder = builder.action(value.parse().map_err(to_js_error)?);
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
            builder = builder.sampler(value.parse().map_err(to_js_error)?);
        }
        if let Some(value) = self.seed {
            builder = builder.seed(value);
        }
        if let Some(value) = self.extra_noise_seed {
            builder = builder.extra_noise_seed(Some(value));
        }
        if let Some(value) = &self.noise_schedule {
            builder = builder.noise_schedule(value.parse().map_err(to_js_error)?);
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
            .references(self.references.clone().unwrap_or_default());
        if let Some(value) = self.estimate_only {
            builder = builder.estimate_only(value);
        }
        builder.build().map_err(to_js_error)
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
                config.timeout = std::time::Duration::from_secs(u64::from(value));
            }
            if let Some(value) = options.vibe_cache_entries {
                config.vibe_cache_entries = value as usize;
            }
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
        use futures_util::StreamExt;
        let request = request.to_core()?;
        let mut stream = self
            .inner
            .stream_generation(&request)
            .await
            .map_err(to_js_error)?;
        let mut events = Vec::new();
        while let Some(event) = stream.next().await {
            let event = event.map_err(to_js_error)?;
            events.push(GenerationEvent {
                event_type: event.event_type,
                sample_index: event.sample_index,
                step_index: event.step_index,
                generation_id: event.generation_id,
                sigma: event.sigma,
                image: Image {
                    filename: event.image.filename,
                    data: event.image.data.into(),
                },
            });
            if max_events.is_some_and(|max| events.len() >= max as usize) {
                break;
            }
        }
        Ok(events)
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
