//! Pure tag/coordinate helpers shared by the request type.
//!
//! Port of the helper half of `novelai_image_mcp.nai.models`.

use rand::Rng;
use unicode_casefold::UnicodeCaseFold;

use crate::error::Error;

/// `extra_noise_seed` is drawn below `2**32 - 7`, as in the Python code.
pub(crate) const EXTRA_NOISE_SEED_BOUND: u32 = u32::MAX - 6;

/// Trim tags and remove case-insensitive duplicates while preserving order.
pub fn normalize_tags<I, S>(values: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for value in values {
        let normalized = value.as_ref().trim();
        let key: String = normalized.case_fold().collect();
        if normalized.is_empty() || !seen.insert(key) {
            continue;
        }
        result.push(normalized.to_owned());
    }
    result
}

/// Split comma/newline negative-prompt text into normalized tags.
pub fn normalize_negative_prompt(value: Option<&str>) -> Vec<String> {
    match value {
        Some(text) if !text.is_empty() => normalize_tags(text.split([',', '\n'])),
        _ => Vec::new(),
    }
}

/// Merge comma-separated tag lists, dropping empty tags and case-insensitive
/// duplicates (first spelling wins), joined with `", "`.
pub fn merge_csv<'a>(values: impl IntoIterator<Item = &'a str>) -> String {
    let mut tags: Vec<&str> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for tag in values
        .into_iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        if seen.insert(tag.case_fold().collect::<String>()) {
            tags.push(tag);
        }
    }
    tags.join(", ")
}

/// A character position for multi-character prompts; **clamps** into
/// `0.1..=0.9` rather than rejecting.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PositionCoord {
    /// Horizontal position, clamped to `0.1..=0.9`.
    pub x: f64,
    /// Vertical position, clamped to `0.1..=0.9`.
    pub y: f64,
}

impl PositionCoord {
    /// Clamped position.
    pub fn new(x: f64, y: f64) -> Self {
        Self {
            x: x.clamp(0.1, 0.9),
            y: y.clamp(0.1, 0.9),
        }
    }
}

/// One additional character's prompt and placement.
///
/// Coordinates outside `0.1..=0.9` are **rejected** (unlike [`PositionCoord`],
/// which clamps) — matching the Python `CharacterPrompt`.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterPrompt {
    /// The character prompt text.
    pub prompt: String,
    /// Per-character undesired content.
    pub negative_prompt: String,
    /// Horizontal position; must be within `0.1..=0.9`.
    pub x: f64,
    /// Vertical position; must be within `0.1..=0.9`.
    pub y: f64,
    /// Whether the character is active.
    pub enabled: bool,
}

impl CharacterPrompt {
    /// A character prompt at the centre.
    pub fn new(prompt: impl Into<String>) -> Self {
        Self {
            prompt: prompt.into(),
            negative_prompt: String::new(),
            x: 0.5,
            y: 0.5,
            enabled: true,
        }
    }

    /// Validate, mirroring the Python `CharacterPrompt.__post_init__`.
    pub fn validate(&self) -> Result<(), Error> {
        if self.prompt.trim().is_empty() {
            return Err(Error::validation("character prompt must not be empty"));
        }
        if !(0.1..=0.9).contains(&self.x) || !(0.1..=0.9).contains(&self.y) {
            return Err(Error::validation(
                "character coordinates must be between 0.1 and 0.9",
            ));
        }
        Ok(())
    }
}

impl super::GenerationRequest {
    /// Fill the img2img/inpaint defaults the reference implementation derives.
    pub(super) fn fill_image_conditioned_defaults<R: Rng + ?Sized>(&mut self, rng: &mut R) {
        self.strength.get_or_insert(0.3);
        self.noise.get_or_insert(0.0);
        if self.extra_noise_seed.is_none() {
            self.extra_noise_seed = Some(rng.random_range(0..EXTRA_NOISE_SEED_BOUND));
        }
    }

    /// Inpainting needs a mask and an inpainting-capable model.
    pub(super) fn check_inpaint_requirements(&self) -> Result<(), Error> {
        if self.mask.is_none() {
            return Err(Error::validation("inpainting requires a mask"));
        }
        if !self.model.is_inpaint() {
            return Err(Error::validation("inpainting requires an inpainting model"));
        }
        Ok(())
    }
}
