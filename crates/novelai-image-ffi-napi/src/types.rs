//! Records crossing the napi boundary: images, events and requests.

use napi::bindgen_prelude::{Buffer, Result};
use napi_derive::napi;

use novelai_image_sdk::spec::GenerationRequestSpec;

use super::to_js_error;

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
#[derive(Debug, Clone, Default, serde::Serialize)]
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
    /// Enable dynamic thresholding (V3 family).
    pub dynamic_thresholding: Option<bool>,
    /// CFG rescale.
    pub cfg_rescale: Option<f64>,
    /// SMEA toggle (V4+ only).
    pub smea: Option<bool>,
    /// Dynamic SMEA toggle (V4+ only).
    pub smea_dynamic: Option<bool>,
    /// Automatic SMEA (V4+ only).
    pub auto_smea: Option<bool>,
    /// Source image, base64.
    pub image: Option<String>,
    /// Inpainting mask, base64.
    pub mask: Option<String>,
    /// img2img denoise strength.
    pub strength: Option<f64>,
    /// img2img noise.
    pub noise: Option<f64>,
    /// Include the source image in the response.
    pub add_original_image: Option<bool>,
    /// ControlNet conditioning strength (`0.1..=2.0`).
    pub controlnet_strength: Option<f64>,
    /// ControlNet conditioning image, base64.
    pub controlnet_condition: Option<String>,
    /// ControlNet model name.
    pub controlnet_model: Option<String>,
    /// Vibe reference tokens.
    pub references: Option<Vec<String>>,
    /// Per-reference information-extracted values.
    pub reference_information: Option<Vec<f64>>,
    /// Per-reference strength values.
    pub reference_strengths: Option<Vec<f64>>,
    /// Additional character prompts (V4+).
    pub character_prompts: Option<Vec<CharacterPrompt>>,
    /// Use per-character coordinates (V4+).
    pub use_coords: Option<bool>,
    /// Preserve prompt order (V4+).
    pub use_order: Option<bool>,
    /// Legacy UC ordering (V4+).
    pub legacy_uc: Option<bool>,
    /// Normalize reference strengths (V4+).
    pub normalize_reference_strengths: Option<bool>,
    /// Reproduce the euler-ancestral quirk (V4+).
    pub deliberate_euler_ancestral_bug: Option<bool>,
    /// Prefer Brownian noise (V4+).
    pub prefer_brownian: Option<bool>,
    /// Skip CFG above this sigma.
    pub skip_cfg_above_sigma: Option<u32>,
    /// Legacy prompt ordering (V3).
    pub legacy: Option<bool>,
    /// Legacy V3 extend prompt handling.
    pub legacy_v3_extend: Option<bool>,
    /// img2img strength applied by inpaint.
    pub inpaint_img2img_strength: Option<u8>,
    /// Apply straight alpha (V5).
    pub straight_alpha: Option<bool>,
    /// Estimate cost only.
    pub estimate_only: Option<bool>,
}

impl GenerationRequest {
    pub(super) fn to_core(&self) -> Result<novelai_image_sdk::models::GenerationRequest> {
        let json = serde_json::to_vec(self)
            .map_err(|e| napi::Error::from_reason(format!("request is not serializable: {e}")))?;
        let spec: GenerationRequestSpec = serde_json::from_slice(&json)
            .map_err(|e| napi::Error::from_reason(format!("request shape mismatch: {e}")))?;
        spec.into_request().map_err(to_js_error)
    }
}

/// One additional character's prompt and placement (V4+).
#[derive(Debug, Clone, Default, serde::Serialize)]
#[napi(object)]
pub struct CharacterPrompt {
    /// The character prompt text (required, non-blank).
    pub prompt: String,
    /// Per-character undesired content.
    pub negative_prompt: String,
    /// Horizontal position (`0.1..=0.9`).
    pub x: f64,
    /// Vertical position (`0.1..=0.9`).
    pub y: f64,
    /// Whether the character is active.
    pub enabled: bool,
}
