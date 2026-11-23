//! Generation request value object, validation and Anlas estimation.
//!
//! Port of the generation subset of `novelai_image_mcp.nai.models`.
//!
//! The Python implementation validates and **mutates** in `__post_init__`;
//! the same checks live in [`GenerationRequest::normalize`], which the builder
//! and the FFI boundary both call.

mod helpers;

pub use helpers::{
    CharacterPrompt, PositionCoord, merge_csv, normalize_negative_prompt, normalize_tags,
};

use rand::Rng;

use crate::constants::{Action, Model, NoiseSchedule, Sampler, quality_tags, uc_presets};
use crate::error::Error;

/// Seed upper bound: an unsigned 32-bit integer.
pub const MAX_SEED: u64 = u32::MAX as u64;

#[cfg(test)]
mod tests;
/// A complete NovelAI generation request, independent of any host.
///
/// Prefer [`GenerationRequest::new`] / [`GenerationRequestBuilder`], which apply
/// the defaults and run [`GenerationRequest::normalize`]. Constructing the
/// struct literally requires calling `normalize` yourself before sending.
#[derive(Debug, Clone)]
pub struct GenerationRequest {
    /// The positive prompt (required, non-blank).
    pub prompt: String,
    /// Base caption for structured-prompt models; defaults to `prompt`.
    pub base_caption: Option<String>,
    /// The model to generate with.
    pub model: Model,
    /// Generation action.
    pub action: Action,
    /// The undesired-content prompt (merged with the UC preset).
    pub negative_prompt: String,
    /// Image width in pixels (rounded up to a multiple of 64).
    pub width: u32,
    /// Image height in pixels (rounded up to a multiple of 64).
    pub height: u32,
    /// Number of images to generate (`1..=8`, resolution-dependent).
    pub n_samples: u32,
    /// Number of sampling steps (`1..=50`).
    pub steps: u32,
    /// Prompt guidance scale.
    pub scale: f64,
    /// Sampling algorithm.
    pub sampler: Sampler,
    /// Seed; `0` lets NovelAI pick one.
    pub seed: u32,
    /// Random seed for img2img/inpaint; filled by `normalize` when absent.
    pub extra_noise_seed: Option<u32>,
    /// Noise schedule.
    pub noise_schedule: NoiseSchedule,
    /// Append the model's quality tags to the prompt.
    pub quality: bool,
    /// Undesired-content preset index (`0..=3`).
    pub uc_preset: u8,
    /// Enable dynamic thresholding (V3 family).
    pub dynamic_thresholding: bool,
    /// CFG rescale (`0.0..=1.0`).
    pub cfg_rescale: f64,
    /// SMEA toggle; `None` omits it (V4+ only).
    pub smea: Option<bool>,
    /// SMEA dynamic toggle; `None` omits it (V4+ only).
    pub smea_dynamic: Option<bool>,
    /// Enable automatic SMEA (V4+ only).
    pub auto_smea: bool,
    /// Source image, base64 (img2img/inpaint).
    pub image: Option<String>,
    /// Inpainting mask, base64.
    pub mask: Option<String>,
    /// img2img denoise strength (`0.01..=0.99`).
    pub strength: Option<f64>,
    /// img2img noise (`0.0..=0.99`).
    pub noise: Option<f64>,
    /// Include the source image in the response.
    pub add_original_image: bool,
    /// ControlNet conditioning strength (`0.1..=2.0`).
    pub controlnet_strength: f64,
    /// ControlNet conditioning image, base64.
    pub controlnet_condition: Option<String>,
    /// ControlNet model name.
    pub controlnet_model: Option<String>,
    /// Vibe-transfer reference tokens.
    pub references: Vec<String>,
    /// Per-reference information-extracted values (`0.01..=1.0`).
    pub reference_information: Vec<f64>,
    /// Per-reference strength values (`0.01..=1.0`).
    pub reference_strengths: Vec<f64>,
    /// Additional character prompts (V4+).
    pub character_prompts: Vec<CharacterPrompt>,
    /// Use per-character coordinates (V4+).
    pub use_coords: bool,
    /// Preserve prompt order (V4+).
    pub use_order: bool,
    /// Legacy UC ordering (V4+).
    pub legacy_uc: bool,
    /// Normalize reference strengths (V4+).
    pub normalize_reference_strengths: bool,
    /// Reproduce the euler-ancestral quirk (V4+).
    pub deliberate_euler_ancestral_bug: bool,
    /// Prefer Brownian noise (V4+).
    pub prefer_brownian: bool,
    /// Skip CFG above this sigma.
    pub skip_cfg_above_sigma: Option<u32>,
    /// Legacy prompt ordering (V3).
    pub legacy: bool,
    /// Legacy V3 extend prompt handling.
    pub legacy_v3_extend: bool,
    /// img2img strength applied by inpaint (`1` for V4.5/V5).
    pub inpaint_img2img_strength: Option<u8>,
    /// Apply straight alpha (V5).
    pub straight_alpha: bool,
    /// Cost-estimation mode: relaxes the image/mask requirements only.
    pub estimate_only: bool,
}

impl Default for GenerationRequest {
    /// Mirrors the Python dataclass defaults (with an empty prompt).
    fn default() -> Self {
        Self {
            prompt: String::new(),
            base_caption: None,
            model: Model::V4_5,
            action: Action::GENERATE,
            negative_prompt: String::new(),
            width: 832,
            height: 1216,
            n_samples: 1,
            steps: 28,
            scale: 5.0,
            sampler: Sampler::EULER_ANCESTRAL,
            seed: 0,
            extra_noise_seed: None,
            noise_schedule: NoiseSchedule::KARRAS,
            quality: true,
            uc_preset: 0,
            dynamic_thresholding: false,
            cfg_rescale: 0.0,
            smea: None,
            smea_dynamic: None,
            auto_smea: false,
            image: None,
            mask: None,
            strength: None,
            noise: None,
            add_original_image: true,
            controlnet_strength: 1.0,
            normalize_reference_strengths: true,
            character_prompts: Vec::new(),
            use_coords: false,
            use_order: true,
            legacy_uc: false,
            deliberate_euler_ancestral_bug: false,
            prefer_brownian: true,
            skip_cfg_above_sigma: None,
            legacy: false,
            legacy_v3_extend: false,
            inpaint_img2img_strength: None,
            straight_alpha: false,
            estimate_only: false,
            controlnet_condition: None,
            controlnet_model: None,
            references: Vec::new(),
            reference_information: Vec::new(),
            reference_strengths: Vec::new(),
        }
    }
}

impl GenerationRequest {
    /// Start building a request for `prompt` with the documented defaults.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(prompt: impl Into<String>) -> GenerationRequestBuilder {
        GenerationRequestBuilder {
            request: Self {
                prompt: prompt.into(),
                ..Self::default()
            },
        }
    }

    /// Validate and normalize, replaying `__post_init__` in its original order
    /// (first error wins) and mutating the derived fields it would.
    ///
    /// Use [`Self::normalize_with_rng`] to make the random
    /// `extra_noise_seed` reproducible.
    pub fn normalize(self) -> Result<Self, Error> {
        self.normalize_with_rng(&mut rand::rng())
    }

    /// Like [`Self::normalize`], but draws the random `extra_noise_seed` from
    /// the supplied RNG.
    pub fn normalize_with_rng<R: Rng + ?Sized>(mut self, rng: &mut R) -> Result<Self, Error> {
        if self.prompt.trim().is_empty() {
            return Err(Error::validation("prompt must not be empty"));
        }
        if self
            .base_caption
            .as_deref()
            .is_some_and(|caption| caption.trim().is_empty())
        {
            return Err(Error::validation(
                "base_caption must not be empty when provided",
            ));
        }
        if !(1..=8).contains(&self.n_samples) {
            return Err(Error::validation("n_samples must be between 1 and 8"));
        }
        if !(1..=50).contains(&self.steps) {
            return Err(Error::validation("steps must be between 1 and 50"));
        }
        if self.uc_preset > 3 {
            return Err(Error::validation("uc_preset must be between 0 and 3"));
        }
        if !(64..=49_152).contains(&self.width) || !(64..=49_152).contains(&self.height) {
            return Err(Error::validation(
                "image dimensions must be between 64 and 49152",
            ));
        }
        self.width = self.width.div_ceil(64) * 64;
        self.height = self.height.div_ceil(64) * 64;
        if self.width * self.height > 3_047_424 {
            return Err(Error::validation(
                "total resolution exceeds NovelAI's limit",
            ));
        }
        if self.n_samples > self.max_samples() {
            return Err(Error::validation(
                "n_samples exceeds the resolution-specific limit",
            ));
        }
        // `estimate_only` relaxes exactly two checks: the image requirement
        // here and the mask/model requirements for inpainting below.
        let image_conditioned = matches!(self.action, Action::IMG2IMG | Action::INPAINT);
        if !self.estimate_only && image_conditioned && self.image.is_none() {
            return Err(Error::validation(
                "image-conditioned actions require an image",
            ));
        }
        if image_conditioned {
            self.fill_image_conditioned_defaults(rng);
        }
        if matches!(
            self.model,
            Model::V4_5 | Model::V4_5_INPAINT | Model::V5 | Model::V5_INPAINT
        ) && self.inpaint_img2img_strength.is_none()
        {
            self.inpaint_img2img_strength = Some(1);
        }
        if self.action == Action::INPAINT && !self.estimate_only {
            self.check_inpaint_requirements()?;
        }
        if self.strength.is_some_and(|s| !(0.01..=0.99).contains(&s)) {
            return Err(Error::validation("strength must be between 0.01 and 0.99"));
        }
        if self.noise.is_some_and(|n| !(0.0..=0.99).contains(&n)) {
            return Err(Error::validation("noise must be between 0 and 0.99"));
        }
        if !(0.1..=2.0).contains(&self.controlnet_strength) {
            return Err(Error::validation(
                "controlnet_strength must be between 0.1 and 2",
            ));
        }
        if !(0.0..=1.0).contains(&self.cfg_rescale) {
            return Err(Error::validation("cfg_rescale must be between 0 and 1"));
        }
        if !self.reference_strengths.is_empty()
            && self.reference_strengths.len() != self.references.len()
        {
            return Err(Error::validation(
                "reference strengths must match references",
            ));
        }
        if !self.reference_information.is_empty()
            && self.reference_information.len() != self.references.len()
        {
            return Err(Error::validation(
                "reference information must match references",
            ));
        }
        if self
            .reference_strengths
            .iter()
            .any(|v| !(0.01..=1.0).contains(v))
        {
            return Err(Error::validation(
                "reference strengths must be between 0.01 and 1",
            ));
        }
        if self
            .reference_information
            .iter()
            .any(|v| !(0.01..=1.0).contains(v))
        {
            return Err(Error::validation(
                "reference information must be between 0.01 and 1",
            ));
        }
        for character in &self.character_prompts {
            character.validate()?;
        }
        Ok(self)
    }

    /// The prompt sent upstream: `prompt` plus the model's quality tags when
    /// `quality` is on.
    pub fn effective_prompt(&self) -> String {
        if !self.quality {
            return self.prompt.clone();
        }
        merge_csv([self.prompt.as_str(), quality_tags(self.model)])
    }

    /// The base caption sent upstream (defaults to the prompt).
    pub fn effective_base_caption(&self) -> String {
        let base = self.base_caption.as_deref().unwrap_or(&self.prompt);
        if !self.quality {
            return base.to_owned();
        }
        merge_csv([base, quality_tags(self.model)])
    }

    /// The negative prompt sent upstream: the UC preset merged with the
    /// user's undesired content.
    pub fn effective_negative_prompt(&self) -> String {
        let preset = uc_presets(self.model)[usize::from(self.uc_preset.min(3))];
        merge_csv([preset, self.negative_prompt.as_str()])
    }

    /// The resolution-dependent sample limit.
    pub fn max_samples(&self) -> u32 {
        let resolution = u64::from(self.width) * u64::from(self.height);
        if resolution <= 512 * 704 {
            return 8;
        }
        if resolution <= 640 * 640 {
            return 6;
        }
        if resolution > 1_024 * 3_072 {
            return 0;
        }
        if resolution <= 1_310_720 {
            return 4;
        }
        if resolution <= 1_572_864 {
            return 2;
        }
        1
    }

    /// Estimate provider Anlas cost using the public web-client formula.
    ///
    /// On V5, Opus free generations are subject to NovelAI's refillable
    /// "battery" limit; once exhausted the API silently bills Anlas, so this
    /// estimate covers the paid cost regardless of quota state.
    pub fn estimate_anlas_cost(&self, opus: bool) -> u32 {
        let normal_portrait = 832u64 * 1_216;
        let normal_square = 1_024u64 * 1_024;
        let mut resolution = (u64::from(self.width) * u64::from(self.height)).max(65_536);
        if normal_portrait < resolution && resolution <= normal_square {
            resolution = normal_portrait;
        }
        let smea_factor = if self.model.is_v4() && self.auto_smea {
            1.2
        } else if self.smea_dynamic == Some(true) {
            1.4
        } else if self.smea == Some(true) {
            1.2
        } else {
            1.0
        };
        let strength = if self.action == Action::IMG2IMG {
            self.strength.unwrap_or(1.0)
        } else {
            1.0
        };
        let base = 2.951_823_174_884_865e-6 * resolution as f64
            + 5.753_298_233_447_344e-7 * resolution as f64 * f64::from(self.steps);
        let per_sample = ((base.ceil() * smea_factor * strength).ceil() as u32).max(2);
        // Python: `free_sample = opus and self.steps <= 28 and resolution <= normal_square`
        // — `normal_square` here is the *possibly rewritten* resolution, so the
        // check runs against the same capped value used in the price formula.
        let free_sample = opus && self.steps <= 28 && resolution <= normal_square;
        per_sample * (self.n_samples - u32::from(free_sample))
    }
}

/// Fluent construction of a validated [`GenerationRequest`].
#[derive(Debug, Clone)]
pub struct GenerationRequestBuilder {
    request: GenerationRequest,
}

macro_rules! builder_setter {
    ($name:ident, $ty:ty, $field:ident) => {
        #[doc = concat!("Set `", stringify!($field), "`.")]
        pub fn $name(mut self, value: $ty) -> Self {
            self.request.$field = value;
            self
        }
    };
}

macro_rules! builder_setter_into {
    ($name:ident, $field:ident) => {
        #[doc = concat!("Set `", stringify!($field), "`.")]
        pub fn $name(mut self, value: impl Into<String>) -> Self {
            self.request.$field = value.into();
            self
        }
    };
}

impl GenerationRequestBuilder {
    builder_setter!(base_caption, Option<String>, base_caption);
    builder_setter!(model, Model, model);
    builder_setter!(action, Action, action);
    builder_setter_into!(negative_prompt, negative_prompt);
    builder_setter!(width, u32, width);
    builder_setter!(height, u32, height);
    builder_setter!(n_samples, u32, n_samples);
    builder_setter!(steps, u32, steps);
    builder_setter!(scale, f64, scale);
    builder_setter!(sampler, Sampler, sampler);
    builder_setter!(seed, u32, seed);
    builder_setter!(extra_noise_seed, Option<u32>, extra_noise_seed);
    builder_setter!(noise_schedule, NoiseSchedule, noise_schedule);
    builder_setter!(quality, bool, quality);
    builder_setter!(uc_preset, u8, uc_preset);
    builder_setter!(dynamic_thresholding, bool, dynamic_thresholding);
    builder_setter!(cfg_rescale, f64, cfg_rescale);
    builder_setter!(smea, Option<bool>, smea);
    builder_setter!(smea_dynamic, Option<bool>, smea_dynamic);
    builder_setter!(auto_smea, bool, auto_smea);
    builder_setter!(image, Option<String>, image);
    builder_setter!(mask, Option<String>, mask);
    builder_setter!(strength, Option<f64>, strength);
    builder_setter!(noise, Option<f64>, noise);
    builder_setter!(add_original_image, bool, add_original_image);
    builder_setter!(controlnet_strength, f64, controlnet_strength);
    builder_setter!(controlnet_condition, Option<String>, controlnet_condition);
    builder_setter!(controlnet_model, Option<String>, controlnet_model);
    builder_setter!(references, Vec<String>, references);
    builder_setter!(reference_information, Vec<f64>, reference_information);
    builder_setter!(reference_strengths, Vec<f64>, reference_strengths);
    builder_setter!(character_prompts, Vec<CharacterPrompt>, character_prompts);
    builder_setter!(use_coords, bool, use_coords);
    builder_setter!(use_order, bool, use_order);
    builder_setter!(legacy_uc, bool, legacy_uc);
    builder_setter!(
        normalize_reference_strengths,
        bool,
        normalize_reference_strengths
    );
    builder_setter!(
        deliberate_euler_ancestral_bug,
        bool,
        deliberate_euler_ancestral_bug
    );
    builder_setter!(prefer_brownian, bool, prefer_brownian);
    builder_setter!(skip_cfg_above_sigma, Option<u32>, skip_cfg_above_sigma);
    builder_setter!(legacy, bool, legacy);
    builder_setter!(legacy_v3_extend, bool, legacy_v3_extend);
    builder_setter!(
        inpaint_img2img_strength,
        Option<u8>,
        inpaint_img2img_strength
    );
    builder_setter!(straight_alpha, bool, straight_alpha);
    builder_setter!(estimate_only, bool, estimate_only);

    /// Validate and normalize.
    pub fn build(self) -> Result<GenerationRequest, Error> {
        self.request.normalize()
    }

    /// Validate and normalize with an explicit RNG (reproducible
    /// `extra_noise_seed`).
    pub fn build_with_rng<R: Rng + ?Sized>(self, rng: &mut R) -> Result<GenerationRequest, Error> {
        self.request.normalize_with_rng(rng)
    }
}
