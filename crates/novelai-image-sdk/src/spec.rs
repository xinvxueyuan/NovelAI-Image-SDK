//! Host-facing request specification.
//!
//! The FFI boundaries (UniFFI records, napi objects) are structurally the same
//! "every field optional" shape. Rather than duplicate the 44-field mapping in
//! each boundary crate, they serialize their record and deserialize it here;
//! [`GenerationRequestSpec::into_request`] then applies the documented defaults
//! and the same validation the reference implementation performs.

use serde::{Deserialize, Serialize};

use crate::constants::{Action, Model, NoiseSchedule, Sampler};
use crate::error::Error;
use crate::models::GenerationRequest;

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
    /// Vibe reference tokens. `null` and absence both mean none, because the
    /// FFI boundaries emit `null` for omitted optional fields.
    pub references: Option<Vec<String>>,
    /// Per-reference information-extracted values.
    pub reference_information: Option<Vec<f64>>,
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
        if let Some(value) = self.cfg_rescale {
            builder = builder.cfg_rescale(value);
        }
        builder = builder
            .image(self.image)
            .mask(self.mask)
            .strength(self.strength)
            .noise(self.noise)
            .references(self.references.unwrap_or_default())
            .reference_information(self.reference_information.unwrap_or_default());
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
}
