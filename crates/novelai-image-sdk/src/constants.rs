//! NovelAI protocol constants and model capability helpers.
//!
//! Direct port of `novelai_image_mcp.nai.constants`. Every table here is
//! asserted against committed golden fixtures (`fixtures/golden/constants.json`)
//! so a transcription error fails CI rather than silently changing the wire
//! payload.
//!
//! Variant names deliberately mirror the reference Python enums (`V4_5`,
//! `DPM_2M`, `PALETTE_SWAP`), so non-camel-case is expected.



use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// API path constants.
///
/// Most paths are rooted at `https://image.novelai.net`. `ANNOTATE` is the one
/// exception: `/ai/annotate-image` was never migrated and remains on the
/// Primary API at `https://api.novelai.net`. `UPSCALE` *has* migrated.
///
/// `/ai/generate-image` returns a ZIP archive (HTTP 201) and serves V3 / Furry
/// legacy models; `/ai/generate-image-stream` returns a length-prefixed
/// MessagePack stream (HTTP 200) and serves V4 / V4.5 / V5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Endpoint {
    /// `POST /user/login`
    Login,
    /// `GET /user/data`
    UserData,
    /// `GET /user/subscription`
    Subscription,
    /// `POST /ai/generate-image`
    Image,
    /// `POST /ai/generate-image-stream`
    ImageStream,
    /// `POST /ai/augment-image`
    Director,
    /// `POST /ai/encode-vibe`
    EncodeVibe,
    /// `POST /ai/upscale`
    Upscale,
    /// `POST /ai/annotate-image` (legacy host only)
    Annotate,
    /// `GET /ai/generate-image/suggest-tags`
    SuggestTags,
}

impl Endpoint {
    /// The path portion of the endpoint.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Login => "/user/login",
            Self::UserData => "/user/data",
            Self::Subscription => "/user/subscription",
            Self::Image => "/ai/generate-image",
            Self::ImageStream => "/ai/generate-image-stream",
            Self::Director => "/ai/augment-image",
            Self::EncodeVibe => "/ai/encode-vibe",
            Self::Upscale => "/ai/upscale",
            Self::Annotate => "/ai/annotate-image",
            Self::SuggestTags => "/ai/generate-image/suggest-tags",
        }
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

macro_rules! wire_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident {
            $(
                $(#[$vmeta:meta])*
                $variant:ident = $wire:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[allow(non_camel_case_types)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[non_exhaustive]
        pub enum $name {
            $(
                $(#[$vmeta])*
                #[doc = concat!("Wire value: `", $wire, "`")]
                #[serde(rename = $wire)]
                $variant
            ),+
        }

        impl $name {
            /// Every variant, in declaration order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The on-the-wire string.
            pub const fn as_wire(self) -> &'static str {
                match self {
                    $(Self::$variant => $wire),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_wire())
            }
        }

        impl FromStr for $name {
            type Err = crate::error::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($wire => Ok(Self::$variant),)+
                    other => Err(crate::error::Error::validation(format!(
                        "invalid {}: {other:?} (expected one of: {})",
                        stringify!($name),
                        Self::ALL
                            .iter()
                            .map(|v| format!("{:?}", v.as_wire()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ))),
                }
            }
        }
    };
}

wire_enum! {
    /// Image-generation models.
    pub enum Model {
        V3 = "nai-diffusion-3",
        V3_INPAINT = "nai-diffusion-3-inpainting",
        FURRY = "nai-diffusion-furry-3",
        FURRY_INPAINT = "nai-diffusion-furry-3-inpainting",
        V4 = "nai-diffusion-4-full",
        V4_INPAINT = "nai-diffusion-4-full-inpainting",
        V4_CURATED = "nai-diffusion-4-curated-preview",
        V4_CURATED_INPAINT = "nai-diffusion-4-curated-inpainting",
        V4_5 = "nai-diffusion-4-5-full",
        V4_5_INPAINT = "nai-diffusion-4-5-full-inpainting",
        V4_5_CURATED = "nai-diffusion-4-5-curated",
        V4_5_CURATED_INPAINT = "nai-diffusion-4-5-curated-inpainting",
        V5 = "nai-diffusion-5-full",
        V5_CURATED = "nai-diffusion-5-curated",
        V5_INPAINT = "nai-diffusion-5-full-inpainting",
    }
}

wire_enum! {
    /// Generation action. `INPAINT` serialises as `infill` for backward
    /// compatibility with the NovelAI API.
    pub enum Action {
        GENERATE = "generate",
        IMG2IMG = "img2img",
        INPAINT = "infill",
    }
}

wire_enum! {
    /// Sampling algorithm.
    pub enum Sampler {
        EULER = "k_euler",
        EULER_ANCESTRAL = "k_euler_ancestral",
        DPM_2S_ANCESTRAL = "k_dpmpp_2s_ancestral",
        DPM_2M = "k_dpmpp_2m",
        DPM_2M_SDE = "k_dpmpp_2m_sde",
        DPM_SDE = "k_dpmpp_sde",
        DDIM = "ddim_v3",
    }
}

wire_enum! {
    /// Noise schedule.
    pub enum NoiseSchedule {
        NATIVE = "native",
        KARRAS = "karras",
        EXPONENTIAL = "exponential",
        POLYEXPONENTIAL = "polyexponential",
    }
}

wire_enum! {
    /// ControlNet conditioning models accepted by `/ai/annotate-image`.
    pub enum ControlNetModel {
        PALETTE_SWAP = "hed",
        FORM_LOCK = "midas",
        SCRIBBLER = "fake_scribble",
        BUILDING_CONTROL = "mlsd",
        LANDSCAPER = "uniformer",
    }
}

wire_enum! {
    /// Director tools accepted by `/ai/augment-image`.
    pub enum DirectorTool {
        LINE_ART = "lineart",
        SKETCH = "sketch",
        BACKGROUND_REMOVAL = "bg-removal",
        DECLUTTER = "declutter",
        COLORIZE = "colorize",
        EMOTION = "emotion",
    }
}

wire_enum! {
    /// Emotions for the `emotion` Director tool.
    pub enum Emotion {
        NEUTRAL = "neutral",
        HAPPY = "happy",
        SAD = "sad",
        ANGRY = "angry",
        SCARED = "scared",
        SURPRISED = "surprised",
        TIRED = "tired",
        EXCITED = "excited",
        NERVOUS = "nervous",
        THINKING = "thinking",
        CONFUSED = "confused",
        SHY = "shy",
        DISGUSTED = "disgusted",
        SMUG = "smug",
        BORED = "bored",
        LAUGHING = "laughing",
        IRRITATED = "irritated",
        AROUSED = "aroused",
        EMBARRASSED = "embarrassed",
        WORRIED = "worried",
        LOVE = "love",
        DETERMINED = "determined",
        HURT = "hurt",
        PLAYFUL = "playful",
    }
}

/// Emotion intensity for the `emotion` Director tool. `0` is strongest, `5`
/// weakest. Serialises as its integer value.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
#[non_exhaustive]
pub enum EmotionLevel {
    /// `0`
    NORMAL = 0,
    /// `1`
    SLIGHTLY_WEAK = 1,
    /// `2`
    WEAK = 2,
    /// `3`
    EVEN_WEAKER = 3,
    /// `4`
    VERY_WEAK = 4,
    /// `5`
    WEAKEST = 5,
}

impl EmotionLevel {
    /// The integer wire value.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

impl TryFrom<u8> for EmotionLevel {
    type Error = crate::error::Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::NORMAL),
            1 => Ok(Self::SLIGHTLY_WEAK),
            2 => Ok(Self::WEAK),
            3 => Ok(Self::EVEN_WEAKER),
            4 => Ok(Self::VERY_WEAK),
            5 => Ok(Self::WEAKEST),
            other => Err(crate::error::Error::validation(format!(
                "invalid EmotionLevel: {other} (expected 0..=5)"
            ))),
        }
    }
}

const V4_MODELS: &[Model] = &[
    Model::V4,
    Model::V4_INPAINT,
    Model::V4_CURATED,
    Model::V4_CURATED_INPAINT,
    Model::V4_5,
    Model::V4_5_INPAINT,
    Model::V4_5_CURATED,
    Model::V4_5_CURATED_INPAINT,
    Model::V5,
    Model::V5_CURATED,
    Model::V5_INPAINT,
];

const V5_MODELS: &[Model] = &[Model::V5, Model::V5_CURATED, Model::V5_INPAINT];

const INPAINT_MODELS: &[Model] = &[
    Model::V3_INPAINT,
    Model::FURRY_INPAINT,
    Model::V4_INPAINT,
    Model::V4_CURATED_INPAINT,
    Model::V4_5_INPAINT,
    Model::V4_5_CURATED_INPAINT,
    Model::V5_INPAINT,
];

/// Models the standalone upscaler accepts.
///
/// Live-verified 2026-10-10 against `POST https://image.novelai.net/ai/upscale`:
/// the V5 line returns 200 while every V3 / V4 / V4.5 variant is rejected with
/// "model ... doesn't support standalone upscaling".
pub const UPSCALE_MODELS: &[Model] = &[Model::V5, Model::V5_CURATED];

impl Model {
    /// True for structured-prompt (V4+) models: V4 / V4.5 / V5 families.
    ///
    /// These use the `/ai/generate-image-stream` endpoint (MessagePack) and the
    /// `v4_prompt` / `v4_negative_prompt` caption payload shape.
    pub fn is_v4(self) -> bool {
        V4_MODELS.contains(&self)
    }

    /// True for NovelAI Diffusion V5 models (Full / Curated / Inpainting).
    pub fn is_v5(self) -> bool {
        V5_MODELS.contains(&self)
    }

    /// True when vibe transfer (reference images) works for this model.
    ///
    /// Vibe transfer is available on V4 / V4.5 only — V3 / Furry and V5 return
    /// `false`.
    pub fn supports_vibe(self) -> bool {
        self.is_v4() && !self.is_v5()
    }

    /// True when the model accepts an inpainting mask.
    pub fn is_inpaint(self) -> bool {
        INPAINT_MODELS.contains(&self)
    }

    /// True when the standalone `/ai/upscale` endpoint accepts this model.
    pub fn supports_upscale(self) -> bool {
        UPSCALE_MODELS.contains(&self)
    }

    /// The NovelAI `params_version` for a model: `4` for V5, `3` otherwise.
    pub fn params_version(self) -> u8 {
        if self.is_v5() { 4 } else { 3 }
    }
}

/// Per-model quality preset appended to the positive prompt when `quality` is on.
pub fn quality_tags(model: Model) -> &'static str {
    match model {
        Model::V5 | Model::V5_INPAINT | Model::V4_5 | Model::V4_5_INPAINT => {
            "very aesthetic, masterpiece, no text"
        }
        Model::V5_CURATED | Model::V4_5_CURATED | Model::V4_5_CURATED_INPAINT => {
            "location, masterpiece, no text, -0.8::feet::, rating:general"
        }
        Model::V4 | Model::V4_INPAINT => "no text, best quality, very aesthetic, absurdres",
        Model::V4_CURATED | Model::V4_CURATED_INPAINT => {
            "rating:general, amazing quality, very aesthetic, absurdres"
        }
        Model::V3 | Model::V3_INPAINT => "best quality, amazing quality, very aesthetic, absurdres",
        Model::FURRY | Model::FURRY_INPAINT => "{best quality}, {amazing quality}",
    }
}

/// Per-model negative-prompt presets indexed by `uc_preset` (`0..=3`).
///
/// The V5 presets reuse the V4.5 texts as a tuning baseline.
pub fn uc_presets(model: Model) -> [&'static str; 4] {
    match model {
        Model::V5_CURATED | Model::V4_5_CURATED | Model::V4_5_CURATED_INPAINT => [
            V45_CURATED_HEAVY,
            V45_CURATED_LIGHT,
            V45_CURATED_FOCUS,
            "",
        ],
        Model::V5 | Model::V5_INPAINT | Model::V4_5 | Model::V4_5_INPAINT => {
            [V45_HEAVY, V45_LIGHT, V45_FURRY, V45_HUMAN]
        }
        Model::V4 | Model::V4_INPAINT => [V4_HEAVY, V4_LIGHT, "", ""],
        Model::V4_CURATED | Model::V4_CURATED_INPAINT => {
            [V4_CURATED_HEAVY, V4_CURATED_LIGHT, "", ""]
        }
        Model::V3 | Model::V3_INPAINT => [V3_HEAVY, V3_LIGHT, V3_HUMAN, ""],
        Model::FURRY | Model::FURRY_INPAINT => [FURRY_HEAVY, FURRY_LIGHT, "", ""],
    }
}

/// V5 `ucPresetId` strings, indexed by `uc_preset` (`0..=3`).
pub const V5_UC_PRESET_IDS: [&str; 4] = ["heavy", "light", "furryFocus", "humanFocus"];

const V45_HEAVY: &str = "nsfw, lowres, artistic error, film grain, scan artifacts, worst quality, \
bad quality, jpeg artifacts, very displeasing, chromatic aberration, dithering, halftone, \
screentone, multiple views, logo, too many watermarks, negative space, blank page";
const V45_LIGHT: &str = "nsfw, lowres, artistic error, scan artifacts, worst quality, bad quality, \
jpeg artifacts, multiple views, very displeasing, too many watermarks, negative space, blank page";
const V45_FURRY: &str = "nsfw, {worst quality}, distracting watermark, unfinished, bad quality, \
{widescreen}, upscale, {sequence}, {{grandfathered content}}, blurred foreground, chromatic \
aberration, sketch, everyone, [sketch background], simple, [flat colors], ych (character), \
outline, multiple scenes, [[horror (theme)]], comic";
const V45_HUMAN: &str =
    "nsfw, lowres, artistic error, film grain, scan artifacts, worst quality, bad quality, jpeg \
artifacts, very displeasing, chromatic aberration, dithering, halftone, screentone, multiple \
views, logo, too many watermarks, negative space, blank page, @_@, mismatched pupils, glowing \
eyes, bad anatomy";
const V45_CURATED_HEAVY: &str = "blurry, lowres, upscaled, artistic error, film grain, scan \
artifacts, worst quality, bad quality, jpeg artifacts, very displeasing, chromatic aberration, \
halftone, multiple views, logo, too many watermarks, negative space, blank page";
const V45_CURATED_LIGHT: &str = "blurry, lowres, upscaled, artistic error, scan artifacts, jpeg \
artifacts, logo, too many watermarks, negative space, blank page";
const V45_CURATED_FOCUS: &str = "blurry, lowres, upscaled, artistic error, film grain, scan \
artifacts, bad anatomy, bad hands, worst quality, bad quality, jpeg artifacts, very displeasing, \
chromatic aberration, halftone, multiple views, logo, too many watermarks, @_@, mismatched \
pupils, glowing eyes, negative space, blank page";
const V4_HEAVY: &str = "blurry, lowres, error, film grain, scan artifacts, worst quality, bad \
quality, jpeg artifacts, very displeasing, chromatic aberration, multiple views, logo, too many \
watermarks";
const V4_LIGHT: &str =
    "blurry, lowres, error, worst quality, bad quality, jpeg artifacts, very displeasing";
const V4_CURATED_HEAVY: &str = "blurry, lowres, error, film grain, scan artifacts, worst quality, \
bad quality, jpeg artifacts, very displeasing, chromatic aberration, logo, dated, signature, \
multiple views, gigantic breasts";
const V4_CURATED_LIGHT: &str = "blurry, lowres, error, worst quality, bad quality, jpeg artifacts, \
very displeasing, logo, dated, signature";
const V3_HEAVY: &str = "lowres, {bad}, error, fewer, extra, missing, worst quality, jpeg \
artifacts, bad quality, watermark, unfinished, displeasing, chromatic aberration, signature, \
extra digits, artistic error, username, scan, [abstract]";
const V3_LIGHT: &str = "lowres, jpeg artifacts, worst quality, watermark, blurry, very displeasing";
const V3_HUMAN: &str = "lowres, {bad}, error, fewer, extra, missing, worst quality, jpeg \
artifacts, bad quality, watermark, unfinished, displeasing, chromatic aberration, signature, \
extra digits, artistic error, username, scan, [abstract], bad anatomy, bad hands, @_@, \
mismatched pupils, heart-shaped pupils, glowing eyes";
const FURRY_HEAVY: &str = "{{worst quality}}, [displeasing], {unusual pupils}, guide lines, \
{{unfinished}}, {bad}, url, artist name, {{tall image}}, mosaic, {sketch page}, comic panel, \
impact (font), [dated], {logo}, ych, {what}, {where is your god now}, {distorted text}, repeated \
text, {floating head}, {1994}, {widescreen}, absolutely everyone, sequence, {compression \
artifacts}, hard translated, {cropped}, {commissioner name}, unknown text, high contrast";
const FURRY_LIGHT: &str = "{worst quality}, guide lines, unfinished, bad, url, tall image, \
widescreen, compression artifacts, unknown text";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_count_and_flags() {
        assert_eq!(Model::ALL.len(), 15);
        assert!(Model::V5.is_v4() && Model::V5.is_v5());
        assert!(!Model::V3.is_v4());
        assert!(Model::V4_5.supports_vibe());
        assert!(!Model::V5.supports_vibe());
        assert!(!Model::V5.is_inpaint());
        assert!(Model::V5_INPAINT.is_inpaint());
        assert!(Model::V5.supports_upscale());
        assert!(!Model::V5_INPAINT.supports_upscale());
        assert_eq!(Model::V5.params_version(), 4);
        assert_eq!(Model::V4_5.params_version(), 3);
    }

    #[test]
    fn human_preset_is_heavy_plus_suffix() {
        assert_eq!(V45_HUMAN, format!("{V45_HEAVY}, @_@, mismatched pupils, glowing eyes, bad anatomy"));
        assert_eq!(
            V3_HUMAN,
            format!("{V3_HEAVY}, bad anatomy, bad hands, @_@, mismatched pupils, heart-shaped pupils, glowing eyes")
        );
    }

    #[test]
    fn wire_roundtrip() {
        for m in Model::ALL {
            assert_eq!(Model::from_str(m.as_wire()).unwrap(), *m);
        }
        assert!(Model::from_str("nope").is_err());
        assert_eq!(Action::INPAINT.as_wire(), "infill");
    }
}
