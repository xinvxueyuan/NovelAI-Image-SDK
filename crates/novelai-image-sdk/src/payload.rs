//! NovelAI V3 / V4+ / V5 wire payload mapping.
//!
//! Port of `novelai_image_mcp.nai.payload.build_generation_payload`.
//!
//! Key insertion order is part of the contract: `serde_json` is built with the
//! `preserve_order` feature and every map is filled in the same order as the
//! Python dict literals, so the compact serialization matches the reference
//! byte-for-byte (asserted by the golden fixtures).

use serde_json::{Map, Value, json};

use crate::constants::V5_UC_PRESET_IDS;
use crate::models::GenerationRequest;

fn center(x: f64, y: f64) -> Value {
    json!({ "x": x, "y": y })
}

/// Serialize every supported generation and conditioning option.
pub fn build_generation_payload(request: &GenerationRequest) -> Value {
    let prompt = request.effective_prompt();
    let negative_prompt = request.effective_negative_prompt();

    let characters: Vec<Value> = request
        .character_prompts
        .iter()
        .map(|item| {
            json!({
                "prompt": item.prompt,
                "uc": item.negative_prompt,
                "center": { "x": item.x, "y": item.y },
                "enabled": item.enabled,
            })
        })
        .collect();
    let char_captions: Vec<Value> = request
        .character_prompts
        .iter()
        .filter(|item| item.enabled)
        .map(|item| {
            json!({
                "char_caption": item.prompt,
                "centers": [center(item.x, item.y)],
            })
        })
        .collect();
    let negative_char_captions: Vec<Value> = request
        .character_prompts
        .iter()
        .filter(|item| item.enabled && !item.negative_prompt.is_empty())
        .map(|item| {
            json!({
                "char_caption": item.negative_prompt,
                "centers": [center(item.x, item.y)],
            })
        })
        .collect();

    // Fixed-order parameters. `serde_json`'s map preserves insertion order.
    let mut parameters = Map::new();
    let mut put = |key: &str, value: Value| {
        parameters.insert(key.to_owned(), value);
    };
    put("width", json!(request.width));
    put("height", json!(request.height));
    put("n_samples", json!(request.n_samples));
    put("steps", json!(request.steps));
    put("scale", json!(request.scale));
    put("sampler", json!(request.sampler.as_wire()));
    put("seed", json!(request.seed));
    put("negative_prompt", json!(negative_prompt));
    put("qualityToggle", json!(request.quality));
    put("params_version", json!(request.model.params_version()));
    put("dynamic_thresholding", json!(request.dynamic_thresholding));
    put("cfg_rescale", json!(request.cfg_rescale));
    put("noise_schedule", json!(request.noise_schedule.as_wire()));
    put("add_original_image", json!(request.add_original_image));
    put("controlnet_strength", json!(request.controlnet_strength));
    put("characterPrompts", Value::Array(characters));
    match request.skip_cfg_above_sigma {
        Some(value) => put("skip_cfg_above_sigma", json!(value)),
        None => put("skip_cfg_above_sigma", Value::Null),
    }
    put("legacy", json!(request.legacy));
    put("legacy_v3_extend", json!(request.legacy_v3_extend));

    let is_v5 = request.model.is_v5();
    let is_v4 = request.model.is_v4();
    if is_v5 {
        // V5 uses string enums where older generations used ints/bools.
        put(
            "ucPresetId",
            json!(V5_UC_PRESET_IDS[usize::from(request.uc_preset.min(3))]),
        );
        put(
            "qualityPresetId",
            json!(if request.quality { "standard" } else { "none" }),
        );
        if request.straight_alpha {
            put("straight_alpha", json!(true));
        }
    } else {
        put("ucPreset", json!(request.uc_preset));
    }

    // Optionals are included only when present, in the Python `optional` order:
    // seven request fields, then `sm`/`sm_dyn` for non-V5 models.
    //
    // V5 clients do not send the SMEA `sm`/`sm_dyn` pair — `autoSmea` is the
    // only SMEA control. Note the V3 path at the bottom overwrites absent
    // `sm`/`sm_dyn` with `false` (appended at the end, matching Python's
    // dict-reassignment semantics).
    let mut optionals: Vec<(&str, Option<Value>)> = vec![
        ("extra_noise_seed", request.extra_noise_seed.map(Value::from)),
        ("image", request.image.as_deref().map(Value::from)),
        ("mask", request.mask.as_deref().map(Value::from)),
        ("strength", request.strength.map(Value::from)),
        ("noise", request.noise.map(Value::from)),
        (
            "controlnet_condition",
            request.controlnet_condition.as_deref().map(Value::from),
        ),
        (
            "controlnet_model",
            request.controlnet_model.as_deref().map(Value::from),
        ),
    ];
    if !is_v5 {
        optionals.push(("sm", request.smea.map(Value::from)));
        optionals.push(("sm_dyn", request.smea_dynamic.map(Value::from)));
    }
    for (key, value) in optionals {
        if let Some(value) = value {
            put(key, value);
        }
    }
    if !request.references.is_empty() {
        put(
            "reference_image_multiple",
            json!(&request.references),
        );
        if !request.reference_information.is_empty() {
            put(
                "reference_information_extracted_multiple",
                json!(&request.reference_information),
            );
        }
        if !request.reference_strengths.is_empty() {
            put(
                "reference_strength_multiple",
                json!(&request.reference_strengths),
            );
        }
    }
    if is_v4 {
        put("autoSmea", json!(request.auto_smea));
        put(
            "normalize_reference_strength_multiple",
            json!(request.normalize_reference_strengths),
        );
        put(
            "deliberate_euler_ancestral_bug",
            json!(request.deliberate_euler_ancestral_bug),
        );
        put("prefer_brownian", json!(request.prefer_brownian));
        let use_coords = request.use_coords || !request.character_prompts.is_empty();
        put("use_coords", json!(use_coords));
        put("legacy_uc", json!(request.legacy_uc));
        put("stream", json!("msgpack"));
        if let Some(strength) = request.inpaint_img2img_strength {
            put("inpaintImg2ImgStrength", json!(strength));
        }
        put(
            "v4_prompt",
            json!({
                "caption": {
                    "base_caption": request.effective_base_caption(),
                    "char_captions": char_captions,
                },
                "use_coords": use_coords,
                "use_order": request.use_order,
            }),
        );
        put(
            "v4_negative_prompt",
            json!({
                "caption": {
                    "base_caption": negative_prompt,
                    "char_captions": negative_char_captions,
                },
                "legacy_uc": request.legacy_uc,
            }),
        );
    } else {
        put("sm", json!(request.smea.unwrap_or(false)));
        put("sm_dyn", json!(request.smea_dynamic.unwrap_or(false)));
    }

    json!({
        "action": request.action.as_wire(),
        "input": prompt,
        "model": request.model.as_wire(),
        "parameters": Value::Object(parameters),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{Action, Model};
    use crate::models::{CharacterPrompt, GenerationRequest};

    fn request() -> GenerationRequest {
        GenerationRequest::new("1girl")
            .negative_prompt("nsfw")
            .build()
            .unwrap()
    }

    #[test]
    fn top_level_keys() {
        let payload = build_generation_payload(&request());
        let object = payload.as_object().unwrap();
        assert_eq!(
            object.keys().collect::<Vec<_>>(),
            ["action", "input", "model", "parameters"]
        );
        assert_eq!(payload["action"], "generate");
        assert_eq!(payload["model"], "nai-diffusion-4-5-full");
        assert_eq!(payload["parameters"]["stream"], "msgpack");
        assert_eq!(payload["parameters"]["params_version"], 3);
    }

    #[test]
    fn v3_has_no_structured_prompt() {
        let req = GenerationRequest::new("1girl").model(Model::V3).build().unwrap();
        let payload = build_generation_payload(&req);
        let params = payload["parameters"].as_object().unwrap();
        assert!(!params.contains_key("v4_prompt"));
        assert!(!params.contains_key("stream"));
        assert_eq!(params["sm"], false);
        assert_eq!(params["sm_dyn"], false);
        assert!(payload["input"].as_str().unwrap().contains("best quality"));
    }

    #[test]
    fn v5_uses_string_presets_and_omits_sm() {
        let req = GenerationRequest::new("1girl")
            .model(Model::V5)
            .uc_preset(3)
            .build()
            .unwrap();
        let payload = build_generation_payload(&req);
        let params = payload["parameters"].as_object().unwrap();
        assert_eq!(params["params_version"], 4);
        assert_eq!(params["ucPresetId"], "humanFocus");
        assert_eq!(params["qualityPresetId"], "standard");
        assert!(!params.contains_key("ucPreset"));
        assert!(!params.contains_key("sm"));
        assert!(!params.contains_key("sm_dyn"));
    }

    #[test]
    fn character_prompts_wire_shape() {
        let req = GenerationRequest::new("1girl")
            .character_prompts(vec![CharacterPrompt {
                prompt: "boy".into(),
                negative_prompt: "glasses".into(),
                x: 0.25,
                y: 0.75,
                enabled: true,
            }])
            .build()
            .unwrap();
        let payload = build_generation_payload(&req);
        let params = &payload["parameters"];
        assert_eq!(params["use_coords"], true);
        assert_eq!(params["characterPrompts"][0]["prompt"], "boy");
        assert_eq!(params["characterPrompts"][0]["uc"], "glasses");
        assert_eq!(params["characterPrompts"][0]["center"]["x"], 0.25);
        assert_eq!(
            params["v4_prompt"]["caption"]["char_captions"][0]["char_caption"],
            "boy"
        );
        assert_eq!(
            params["v4_negative_prompt"]["caption"]["char_captions"][0]["char_caption"],
            "glasses"
        );
    }

    #[test]
    fn img2img_carries_image_fields() {
        let req = GenerationRequest::new("1girl")
            .action(Action::IMG2IMG)
            .image(Some("aGk=".into()))
            .build()
            .unwrap();
        let payload = build_generation_payload(&req);
        let params = &payload["parameters"];
        assert_eq!(params["image"], "aGk=");
        assert_eq!(params["strength"], 0.3);
        assert_eq!(params["noise"], 0.0);
        assert!(params["extra_noise_seed"].is_u64());
    }

    #[test]
    fn v45_keeps_int_presets() {
        let req = GenerationRequest::new("1girl")
            .model(Model::V4_5)
            .uc_preset(2)
            .build()
            .unwrap();
        let payload = build_generation_payload(&req);
        let params = payload["parameters"].as_object().unwrap();
        assert_eq!(params["ucPreset"], 2);
        assert!(!params.contains_key("ucPresetId"));
        assert_eq!(params["params_version"], 3);
    }
}
