//! Client behavior against a local mock server.
//!
//! Mirrors `apps/server/tests/test_client.py` in the reference repository.
//! Requests use `BrowserProfile::chrome()`; the mock serves plain HTTP, which
//! `wreq` handles without TLS emulation issues.

use novelai_image_sdk::auth::Credentials;
use novelai_image_sdk::client::NovelAiClient;
use novelai_image_sdk::config::ClientConfig;
use novelai_image_sdk::constants::{Action, ControlNetModel, DirectorTool, Emotion, EmotionLevel, Model};
use novelai_image_sdk::imaging::TINY_PNG;
use novelai_image_sdk::models::GenerationRequest;
use serde_json::Value;
use wiremock::matchers::{body_json, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(server: &MockServer) -> NovelAiClient {
    // The legacy host is only exercised by the annotate test; point it at the
    // same mock server.
    let config = ClientConfig::new(Credentials::token("test-token"))
        .with_image_base_url(server.uri())
        .with_account_base_url(server.uri());
    let config = ClientConfig {
        legacy_image_base_url: server.uri(),
        ..config
    };
    NovelAiClient::new(config).expect("client")
}

/// Exact header-value matcher.
///
/// `wiremock::matchers::header` splits values on commas, which corrupts the
/// Chrome User-Agent ("KHTML, like Gecko") and `Priority` ("u=1, i").
struct ExactHeader(&'static str, &'static str);

impl wiremock::Match for ExactHeader {
    fn matches(&self, request: &wiremock::Request) -> bool {
        request
            .headers
            .get_all(self.0)
            .iter()
            .any(|value| value.to_str().is_ok_and(|v| v == self.1))
    }
}

/// `Result::unwrap_err` needs `Debug`, which a boxed stream does not have.
fn err_stream<T>(result: Result<T, novelai_image_sdk::error::Error>) -> novelai_image_sdk::error::Error {
    match result {
        Ok(_) => panic!("expected an error"),
        Err(err) => err,
    }
}

fn png_frame(frames: &[Vec<u8>]) -> Vec<u8> {
    let mut body = Vec::new();
    for payload in frames {
        body.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        body.extend_from_slice(payload);
    }
    body
}

fn final_event(image: &[u8]) -> Vec<u8> {
    let value = rmpv::Value::Map(vec![
        ("event_type".into(), "final".into()),
        ("samp_ix".into(), 0.into()),
        ("step_ix".into(), 1.into()),
        ("gen_id".into(), "g".into()),
        ("sigma".into(), 0.0.into()),
        ("image".into(), rmpv::Value::Binary(image.to_vec())),
    ]);
    let mut out = Vec::new();
    rmpv::encode::write_value(&mut out, &value).unwrap();
    out
}

#[tokio::test]
async fn bearer_token_sent_and_generation_payload_matches() {
    let server = MockServer::start().await;
    let body = png_frame(&[final_event(TINY_PNG)]);
    Mock::given(method("POST"))
        .and(path("/ai/generate-image-stream"))
        .and(header("authorization", "Bearer test-token"))
        .and(ExactHeader(
            "user-agent",
            novelai_image_sdk::browser::CHROME_USER_AGENT,
        ))
        .and(ExactHeader("priority", "u=1, i"))
        .and(header("referer", "https://novelai.net/"))
        .and(header("sec-fetch-site", "same-site"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .expect(1)
        .mount(&server)
        .await;

    let out = client(&server)
        .generate(&GenerationRequest::new("1girl").build().unwrap())
        .await
        .expect("generate");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].data, TINY_PNG);
    assert_eq!(out[0].filename, "sample-0-final.png");
}

#[tokio::test]
async fn v3_uses_zip_endpoint_and_parses_archive() {
    let server = MockServer::start().await;
    let mut buffer = Vec::new();
    {
        use std::io::Write;
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));
        writer
            .start_file("image_0.png", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(TINY_PNG).unwrap();
        writer.finish().unwrap();
    }
    Mock::given(method("POST"))
        .and(path("/ai/generate-image"))
        .respond_with(ResponseTemplate::new(201).set_body_bytes(buffer))
        .expect(1)
        .mount(&server)
        .await;

    let out = client(&server)
        .generate(
            &GenerationRequest::new("1girl")
                .model(Model::V3)
                .build()
                .unwrap(),
        )
        .await
        .expect("generate");
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].filename, "image_0.png");
}

#[tokio::test]
async fn upscale_body_is_exact() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ai/upscale"))
        .and(body_json(serde_json::json!({
            "image": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg==",
            "model": "nai-diffusion-5-full",
            "declared_blur_sigma": 0
        })))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(TINY_PNG))
        .expect(1)
        .mount(&server)
        .await;

    let out = client(&server).upscale(TINY_PNG, 4, None).await.expect("upscale");
    assert_eq!(out.filename, "upscaled.png");
    assert_eq!(out.data, TINY_PNG);
}

#[tokio::test]
async fn annotate_hits_legacy_host() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ai/annotate-image"))
        .and(body_json(serde_json::json!({
            "model": "hed",
            "parameters": {"image": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg=="}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(TINY_PNG))
        .expect(1)
        .mount(&server)
        .await;

    let out = client(&server)
        .annotate(TINY_PNG, ControlNetModel::PALETTE_SWAP)
        .await
        .expect("annotate");
    assert_eq!(out.filename, "hed.png");
}

#[tokio::test]
async fn director_emotion_composes_prompt() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ai/augment-image"))
        .and(body_json(serde_json::json!({
            "req_type": "emotion",
            "width": 1,
            "height": 1,
            "image": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg==",
            "prompt": "happy;;a smile,",
            "defry": 2
        })))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(TINY_PNG))
        .expect(1)
        .mount(&server)
        .await;

    let out = client(&server)
        .director(
            DirectorTool::EMOTION,
            TINY_PNG,
            "a smile",
            0,
            Some(Emotion::HAPPY),
            EmotionLevel::WEAK,
        )
        .await
        .expect("director");
    assert_eq!(out.filename, "emotion.png");
}

#[tokio::test]
async fn suggest_tags_is_a_get_with_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ai/generate-image/suggest-tags"))
        .and(query_param("model", "nai-diffusion-4-5-full"))
        .and(query_param("prompt", "1girl, smi"))
        .and(query_param("lang", "en"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "tags": [{"tag": "1girl", "count": 10000, "confidence": 0.79}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let tags = client(&server)
        .suggest_tags("1girl, smi", Model::V4_5, "en")
        .await
        .expect("tags");
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0]["tag"], "1girl");
}

#[tokio::test]
async fn vibe_encoding_is_cached() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/ai/encode-vibe"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"encoded-bytes"))
        .expect(1) // second call must hit the cache
        .mount(&server)
        .await;

    let client = client(&server);
    let first = client
        .encode_vibe("ref", 0.5, Model::V4_5)
        .await
        .expect("encode");
    let second = client
        .encode_vibe("ref", 0.5, Model::V4_5)
        .await
        .expect("cached");
    assert_eq!(first, second);
    assert_eq!(first, "ZW5jb2RlZC1ieXRlcw==");
}

#[tokio::test]
async fn login_is_called_once_for_password_credentials() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/user/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "accessToken": "derived-token"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/user/data"))
        .and(header("authorization", "Bearer derived-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"tier": 3})))
        .expect(2)
        .mount(&server)
        .await;

    let config = ClientConfig::new(Credentials::password("user@example.com", "hunter2"))
        .with_image_base_url(server.uri())
        .with_account_base_url(server.uri());
    let client = NovelAiClient::new(config).expect("client");
    assert_eq!(client.get_user_data().await.unwrap()["tier"], 3);
    assert_eq!(client.get_user_data().await.unwrap()["tier"], 3);
}

#[tokio::test]
async fn statuses_map_to_typed_errors() {
    let cases = [
        (400u16, "NovelAI HTTP 400: boom [code 400: invalid request (validation failed)]"),
        (401u16, "NovelAI HTTP 401: boom [code 401: authentication failed (missing, invalid, or expired credentials)]"),
        (402u16, "NovelAI HTTP 402: boom [code 402: active subscription or Anlas required]"),
        (429u16, "NovelAI HTTP 429: boom [code 429: rate limit or concurrency limit reached]"),
        (500u16, "NovelAI HTTP 500: boom [code 500]"),
    ];
    for (status, expected) in cases {
        // A fresh server per case, since matching is first-mounted-wins.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/user/subscription"))
            .respond_with(ResponseTemplate::new(status).set_body_json(serde_json::json!({
                "statusCode": status,
                "message": "boom"
            })))
            .mount(&server)
            .await;
        let err = client(&server).get_subscription().await.unwrap_err();
        assert_eq!(err.to_string(), expected);
        assert_eq!(
            err.code().map(ToString::to_string),
            Some(status.to_string())
        );
    }
}

#[tokio::test]
async fn img2img_sends_image_and_strength() {
    let server = MockServer::start().await;
    let body = png_frame(&[final_event(TINY_PNG)]);
    Mock::given(method("POST"))
        .and(path("/ai/generate-image-stream"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&server)
        .await;

    let request = GenerationRequest::new("1girl")
        .action(Action::IMG2IMG)
        .image(Some(
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg==".into(),
        ))
        .build()
        .unwrap();
    let client = client(&server);
    client.generate(&request).await.expect("generate");

    let requests = server.received_requests().await.unwrap_or_default();
    let payload: Value =
        serde_json::from_slice(&requests[0].body).expect("request body is JSON");
    let params = &payload["parameters"];
    assert_eq!(params["image"], request.image.clone().unwrap());
    assert_eq!(params["strength"], 0.3);
    assert!(params["extra_noise_seed"].is_u64());
}

#[tokio::test]
async fn streaming_yields_events_and_checks_truncation() {
    let server = MockServer::start().await;
    let body = png_frame(&[final_event(TINY_PNG), final_event(TINY_PNG)]);
    Mock::given(method("POST"))
        .and(path("/ai/generate-image-stream"))
        .and(header("accept", "application/x-msgpack"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body))
        .mount(&server)
        .await;

    let client = client(&server);
    let request = GenerationRequest::new("1girl").build().unwrap();
    let mut stream = client.stream_generation(&request).await.expect("stream");
    let mut events = Vec::new();
    while let Some(event) = futures_util::StreamExt::next(&mut stream).await {
        events.push(event.expect("event"));
    }
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "final");
}

#[tokio::test]
async fn streaming_rejects_v3() {
    let client = NovelAiClient::new(ClientConfig::new(Credentials::token("t"))).unwrap();
    let request = GenerationRequest::new("1girl")
        .model(Model::V3)
        .build()
        .unwrap();
    let err = err_stream(client.stream_generation(&request).await);
    assert!(err.to_string().contains("V4+"));
}

#[tokio::test]
async fn v5_rejects_vibe_references() {
    let client = NovelAiClient::new(ClientConfig::new(Credentials::token("t"))).unwrap();
    let request = GenerationRequest::new("1girl")
        .model(Model::V5)
        .references(vec!["token".into()])
        .build()
        .unwrap();
    let err = client.generate(&request).await.unwrap_err();
    assert!(err.to_string().contains("not supported on V5"));
}
