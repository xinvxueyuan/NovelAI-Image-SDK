use super::*;

#[test]
fn models_and_functions_expose_the_catalog() {
    let models = list_models();
    assert_eq!(models.len(), 15);
    assert!(
        models
            .iter()
            .any(|m| m.id == "nai-diffusion-5-full" && m.is_v5)
    );
    assert_eq!(
        error_explanation(402).unwrap(),
        "active subscription or Anlas required"
    );
    assert_eq!(browser_headers().len(), 13);
    assert_eq!(version(), novelai_image_sdk::VERSION);
}

#[test]
fn request_resolution_matches_the_core() {
    let client = NovelAiClient::new(
        Credentials {
            token: Some("t".into()),
            username: None,
            password: None,
        },
        None,
    )
    .expect("client");
    let resolved = client
        .resolve_request(GenerationRequest {
            prompt: "1girl".into(),
            width: Some(100),
            ..Default::default()
        })
        .expect("resolve");
    assert_eq!(resolved.width, 128);
    assert!(resolved.effective_prompt.starts_with("1girl"));
    let payload: serde_json::Value = serde_json::from_str(&resolved.payload_json).unwrap();
    assert_eq!(payload["model"], "nai-diffusion-4-5-full");
}

#[test]
fn errors_keep_preformatted_messages() {
    let err = SdkError::from(novelai_image_sdk::error::Error::Authentication {
        message: "NovelAI HTTP 401: Unauthorized".into(),
        code: Some(novelai_image_sdk::error::ErrorCode::Http(401)),
        explanation: Some("authentication failed".into()),
    });
    assert!(err.to_string().contains("NovelAI HTTP 401"));
}
