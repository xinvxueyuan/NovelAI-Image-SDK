use super::*;

#[test]
fn upscale_rejects_non_4x_and_unsupported_models() {
    // The validation runs before any I/O, so a config-less client is fine.
    // (Credential validation requires a token.)
    let client = NovelAiClient::new(ClientConfig::new(crate::auth::Credentials::token("t")))
        .expect("client");
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let err = client
            .upscale(crate::imaging::TINY_PNG, 2, None)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("4x only"));
        let err = client
            .upscale(crate::imaging::TINY_PNG, 4, Some(Model::V4_5))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("does not support standalone"));
    });
}
