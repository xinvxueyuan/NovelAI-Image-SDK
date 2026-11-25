//! Host-facing request specification.
//!
//! The FFI boundaries (UniFFI records, napi objects) are structurally the same
//! "every field optional" shape. Rather than duplicate the 45-field mapping in
//! each boundary crate, they serialize their record and deserialize it here;
//! [`GenerationRequestSpec::into_request`] then applies the documented defaults
//! and the same validation the reference implementation performs.

use serde::{Deserialize, Serialize};

use crate::constants::{Action, Model, NoiseSchedule, Sampler};
use crate::error::Error;
use crate::models::{CharacterPrompt, GenerationRequest};

/// One additional character's prompt, as supplied by a host binding.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CharacterPromptSpec {
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

impl Default for CharacterPromptSpec {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            negative_prompt: String::new(),
            x: 0.5,
            y: 0.5,
            enabled: true,
        }
    }
}

impl CharacterPromptSpec {
    fn into_prompt(self) -> CharacterPrompt {
        CharacterPrompt {
            prompt: self.prompt,
            negative_prompt: self.negative_prompt,
            x: self.x,
            y: self.y,
            enabled: self.enabled,
        }
    }
}

/// A generation request with every field optional.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GenerationRequestSpec {
    /// The positive prompt (required).
    pub prompt: String,
    /// Base caption for structured-prompt models.
    pub base_caption: Option<String>,
    /// Model wire id; defaults to `nai-diffusion-4-5-full`.
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
    /// Vibe reference tokens. `null` and absence both mean none, because the
    /// FFI boundaries emit `null` for omitted optional fields.
    pub references: Option<Vec<String>>,
    /// Per-reference information-extracted values.
    pub reference_information: Option<Vec<f64>>,
    /// Per-reference strength values.
    pub reference_strengths: Option<Vec<f64>>,
    /// Additional character prompts (V4+).
    pub character_prompts: Option<Vec<CharacterPromptSpec>>,
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

impl GenerationRequestSpec {
    /// Apply defaults and validate, producing the request the client sends.
    pub fn into_request(self) -> Result<GenerationRequest, Error> {
        let mut builder = GenerationRequest::new(self.prompt.clone());
        if let Some(value) = self.base_caption.clone() {
            builder = builder.base_caption(Some(value));
        }
        builder = self.apply_vocabulary(builder)?;
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
        if let Some(value) = self.seed {
            builder = builder.seed(value);
        }
        if let Some(value) = self.extra_noise_seed {
            builder = builder.extra_noise_seed(Some(value));
        }
        if let Some(value) = self.quality {
            builder = builder.quality(value);
        }
        if let Some(value) = self.uc_preset {
            builder = builder.uc_preset(value);
        }
        if let Some(value) = self.dynamic_thresholding {
            builder = builder.dynamic_thresholding(value);
        }
        if let Some(value) = self.cfg_rescale {
            builder = builder.cfg_rescale(value);
        }
        if let Some(value) = self.smea {
            builder = builder.smea(Some(value));
        }
        if let Some(value) = self.smea_dynamic {
            builder = builder.smea_dynamic(Some(value));
        }
        if let Some(value) = self.auto_smea {
            builder = builder.auto_smea(value);
        }
        builder = builder
            .image(self.image)
            .mask(self.mask)
            .strength(self.strength)
            .noise(self.noise)
            .references(self.references.unwrap_or_default())
            .reference_information(self.reference_information.unwrap_or_default())
            .reference_strengths(self.reference_strengths.unwrap_or_default())
            .character_prompts(
                self.character_prompts
                    .unwrap_or_default()
                    .into_iter()
                    .map(CharacterPromptSpec::into_prompt)
                    .collect(),
            );
        if let Some(value) = self.add_original_image {
            builder = builder.add_original_image(value);
        }
        if let Some(value) = self.controlnet_strength {
            builder = builder.controlnet_strength(value);
        }
        builder = builder
            .controlnet_condition(self.controlnet_condition)
            .controlnet_model(self.controlnet_model);
        if let Some(value) = self.use_coords {
            builder = builder.use_coords(value);
        }
        if let Some(value) = self.use_order {
            builder = builder.use_order(value);
        }
        if let Some(value) = self.legacy_uc {
            builder = builder.legacy_uc(value);
        }
        if let Some(value) = self.normalize_reference_strengths {
            builder = builder.normalize_reference_strengths(value);
        }
        if let Some(value) = self.deliberate_euler_ancestral_bug {
            builder = builder.deliberate_euler_ancestral_bug(value);
        }
        if let Some(value) = self.prefer_brownian {
            builder = builder.prefer_brownian(value);
        }
        if let Some(value) = self.skip_cfg_above_sigma {
            builder = builder.skip_cfg_above_sigma(Some(value));
        }
        if let Some(value) = self.legacy {
            builder = builder.legacy(value);
        }
        if let Some(value) = self.legacy_v3_extend {
            builder = builder.legacy_v3_extend(value);
        }
        if let Some(value) = self.inpaint_img2img_strength {
            builder = builder.inpaint_img2img_strength(Some(value));
        }
        if let Some(value) = self.straight_alpha {
            builder = builder.straight_alpha(value);
        }
        if let Some(value) = self.estimate_only {
            builder = builder.estimate_only(value);
        }
        builder.build()
    }

    /// Apply the enum-valued fields (they need `FromStr`).
    fn apply_vocabulary(
        &self,
        mut builder: crate::models::GenerationRequestBuilder,
    ) -> Result<crate::models::GenerationRequestBuilder, Error> {
        if let Some(value) = &self.model {
            builder = builder.model(value.parse::<Model>()?);
        }
        if let Some(value) = &self.action {
            builder = builder.action(value.parse::<Action>()?);
        }
        if let Some(value) = &self.negative_prompt {
            builder = builder.negative_prompt(value.clone());
        }
        if let Some(value) = &self.sampler {
            builder = builder.sampler(value.parse::<Sampler>()?);
        }
        if let Some(value) = &self.noise_schedule {
            builder = builder.noise_schedule(value.parse::<NoiseSchedule>()?);
        }
        Ok(builder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_and_rounding_apply() {
        let spec: GenerationRequestSpec =
            serde_json::from_value(serde_json::json!({"prompt": "1girl", "width": 100})).unwrap();
        let request = spec.into_request().unwrap();
        assert_eq!((request.width, request.height), (128, 1216));
        assert_eq!(request.model, Model::V4_5);
    }

    #[test]
    fn null_sequences_mean_none() {
        // The napi boundary serializes omitted optional vectors as `null`.
        let spec: GenerationRequestSpec = serde_json::from_value(serde_json::json!({
            "prompt": "1girl",
            "references": null,
            "reference_information": null
        }))
        .unwrap();
        let request = spec.into_request().unwrap();
        assert_eq!(request.references.len(), 0);
        assert_eq!(request.reference_information.len(), 0);
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let result: Result<GenerationRequestSpec, _> =
            serde_json::from_value(serde_json::json!({"prompt": "x", "widht": 832}));
        assert!(result.is_err(), "typos must not be silently ignored");
    }

    #[test]
    fn invalid_vocabulary_is_an_error() {
        let spec = GenerationRequestSpec {
            prompt: "x".into(),
            model: Some("nope".into()),
            ..Default::default()
        };
        assert!(
            spec.into_request()
                .unwrap_err()
                .to_string()
                .contains("invalid Model")
        );
    }

    #[test]
    fn every_advanced_field_survives_into_the_request() {
        let spec: GenerationRequestSpec = serde_json::from_value(serde_json::json!({
            "prompt": "1girl",
            "model": "nai-diffusion-4-5-full",
            "dynamic_thresholding": true,
            "smea": true,
            "smea_dynamic": false,
            "auto_smea": true,
            "add_original_image": false,
            "controlnet_strength": 1.5,
            "controlnet_condition": "Y29uZGl0aW9u",
            "controlnet_model": "hed",
            "reference_strengths": [0.5],
            "references": ["cmVm"],
            "reference_information": [0.75],
            "character_prompts": [{
                "prompt": "boy",
                "negative_prompt": "glasses",
                "x": 0.25,
                "y": 0.75,
                "enabled": true
            }],
            "use_coords": true,
            "use_order": false,
            "legacy_uc": true,
            "normalize_reference_strengths": false,
            "deliberate_euler_ancestral_bug": true,
            "prefer_brownian": false,
            "skip_cfg_above_sigma": 19,
            "legacy": false,
            "legacy_v3_extend": true,
            "inpaint_img2img_strength": 1,
            "straight_alpha": true
        }))
        .unwrap();
        let request = spec.into_request().unwrap();
        assert!(request.dynamic_thresholding);
        assert_eq!(request.smea, Some(true));
        assert_eq!(request.smea_dynamic, Some(false));
        assert!(request.auto_smea);
        assert!(!request.add_original_image);
        assert!((request.controlnet_strength - 1.5).abs() < f64::EPSILON);
        assert_eq!(
            request.controlnet_condition.as_deref(),
            Some("Y29uZGl0aW9u")
        );
        assert_eq!(request.controlnet_model.as_deref(), Some("hed"));
        assert_eq!(request.reference_strengths, vec![0.5]);
        assert_eq!(request.reference_information, vec![0.75]);
        assert_eq!(request.character_prompts.len(), 1);
        assert_eq!(request.character_prompts[0].prompt, "boy");
        assert!(request.use_coords);
        assert!(!request.use_order);
        assert!(request.legacy_uc);
        assert!(!request.normalize_reference_strengths);
        assert!(request.deliberate_euler_ancestral_bug);
        assert!(!request.prefer_brownian);
        assert_eq!(request.skip_cfg_above_sigma, Some(19));
        assert!(request.legacy_v3_extend);
        assert_eq!(request.inpaint_img2img_strength, Some(1));
        assert!(request.straight_alpha);
    }
}
