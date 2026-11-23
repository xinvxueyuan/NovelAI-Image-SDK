use super::*;

#[test]
fn defaults_and_rounding() {
    let req = GenerationRequest::new("a cat").build().unwrap();
    assert_eq!((req.width, req.height), (832, 1216));
    let req = GenerationRequest::new("a cat")
        .width(100)
        .height(100)
        .build()
        .unwrap();
    assert_eq!((req.width, req.height), (128, 128));
}

#[test]
fn validation_messages_match_python() {
    let cases: &[(&str, fn(&mut GenerationRequestBuilder))] =
        &[("prompt must not be empty", |b| {
            *b = std::mem::replace(b, GenerationRequest::new("x")).negative_prompt(String::new());
        })];
    let _ = cases;

    let err = GenerationRequest::new("  ").build().unwrap_err();
    assert_eq!(err.to_string(), "prompt must not be empty");

    let err = GenerationRequest::new("x").steps(0).build().unwrap_err();
    assert_eq!(err.to_string(), "steps must be between 1 and 50");

    let err = GenerationRequest::new("x").width(32).build().unwrap_err();
    assert_eq!(
        err.to_string(),
        "image dimensions must be between 64 and 49152"
    );

    let err = GenerationRequest::new("x")
        .n_samples(9)
        .build()
        .unwrap_err();
    assert_eq!(err.to_string(), "n_samples must be between 1 and 8");

    let err = GenerationRequest::new("x")
        .action(Action::IMG2IMG)
        .build()
        .unwrap_err();
    assert_eq!(
        err.to_string(),
        "image-conditioned actions require an image"
    );
}

#[test]
fn image_conditioned_defaults() {
    let req = GenerationRequest::new("x")
        .action(Action::IMG2IMG)
        .image(Some("aGk=".into()))
        .build()
        .unwrap();
    assert_eq!(req.strength, Some(0.3));
    assert_eq!(req.noise, Some(0.0));
    let seed = req.extra_noise_seed.expect("filled");
    assert!(seed < crate::models::helpers::EXTRA_NOISE_SEED_BOUND);
}

#[test]
fn v5_inpaint_strength() {
    let req = GenerationRequest::new("x")
        .model(Model::V5)
        .build()
        .unwrap();
    assert_eq!(req.inpaint_img2img_strength, Some(1));
    let req = GenerationRequest::new("x")
        .model(Model::V4_5)
        .build()
        .unwrap();
    assert_eq!(req.inpaint_img2img_strength, Some(1));
    let req = GenerationRequest::new("x")
        .model(Model::V3)
        .build()
        .unwrap();
    assert_eq!(req.inpaint_img2img_strength, None);
}

#[test]
fn efficient_prompt_merging() {
    let req = GenerationRequest::new("1girl, 1girl").build().unwrap();
    assert_eq!(
        req.effective_prompt(),
        "1girl, very aesthetic, masterpiece, no text"
    );
    let req = GenerationRequest::new("1girl")
        .negative_prompt("nsfw, blurry, NSFW")
        .build()
        .unwrap();
    // The UC preset is *prepended*; user tags are appended, deduped
    // case-insensitively. "nsfw" is already in the preset, so only
    // "blurry" survives from the user prompt.
    let negative = req.effective_negative_prompt();
    assert!(negative.ends_with(", blurry"), "got: {negative}");
    assert_eq!(
        negative.matches("nsfw").count(),
        1,
        "dedupe failed: {negative}"
    );
}

#[test]
fn anlas_estimate_formula() {
    let req = GenerationRequest::new("x").build().unwrap();
    assert_eq!(req.estimate_anlas_cost(false), 20);
    let req = GenerationRequest::new("x").n_samples(4).build().unwrap();
    assert_eq!(req.estimate_anlas_cost(false), 80);
    let req = GenerationRequest::new("x").build().unwrap();
    assert_eq!(req.estimate_anlas_cost(true), 0);
}

#[test]
fn max_samples_bands() {
    let bands = [
        (512, 704, 8),
        (640, 640, 6),
        (1024, 1280, 4),
        (1024, 1536, 2),
        (1472, 1472, 1),
    ];
    for (w, h, expected) in bands {
        let req = GenerationRequest::new("x")
            .width(w)
            .height(h)
            .build()
            .unwrap();
        assert_eq!(req.max_samples(), expected, "{w}x{h}");
    }
}
