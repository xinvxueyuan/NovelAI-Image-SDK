//! Types crossing the UniFFI boundary.
//!
//! Records are plain data; the conversions that matter live beside the client
//! impl in [`super`].

use novelai_image_sdk::auth::Credentials as CoreCredentials;

use super::SdkError;

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
    pub(super) fn to_core(&self) -> Result<CoreCredentials, SdkError> {
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

impl ClientOptions {
    pub(super) fn to_overrides(&self) -> novelai_image_sdk::config::ConfigOverrides {
        novelai_image_sdk::config::ConfigOverrides {
            image_base_url: self.image_base_url.clone(),
            account_base_url: self.account_base_url.clone(),
            legacy_image_base_url: self.legacy_image_base_url.clone(),
            timeout: self.timeout_secs.map(std::time::Duration::from_secs),
            vibe_cache_entries: self.vibe_cache_entries.map(|value| value as usize),
        }
    }
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
    pub references: Option<Vec<String>>,
    /// Per-reference information-extracted values.
    pub reference_information: Option<Vec<f64>>,
    /// Estimate cost only (relaxes the image/mask requirements).
    pub estimate_only: Option<bool>,
}

impl GenerationRequest {
    pub(super) fn to_core(&self) -> Result<novelai_image_sdk::models::GenerationRequest, SdkError> {
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
            .references(self.references.clone().unwrap_or_default())
            .reference_information(self.reference_information.clone().unwrap_or_default());
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
