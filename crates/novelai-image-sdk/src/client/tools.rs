//! Tools, account and transport plumbing for [`super::NovelAiClient`].
//!
//! Generation and auth live in the parent module.

use serde_json::{Map, Value};

use crate::constants::{ControlNetModel, DirectorTool, Emotion, EmotionLevel, Endpoint, Model};
use crate::error::Error;
use crate::imaging::parse_image;
use crate::response::NovelAIImage;

use super::{first_image, json_object};

impl super::NovelAiClient {
    // ── tools ────────────────────────────────────────────────────────────

    /// Run a Director tool (`/ai/augment-image`).
    pub async fn director(
        &self,
        tool: DirectorTool,
        image: &[u8],
        prompt: &str,
        defry: u32,
        emotion: Option<Emotion>,
        emotion_level: EmotionLevel,
    ) -> Result<NovelAIImage, Error> {
        let parsed = parse_image(image)?;
        let (prompt, defry) = if tool == DirectorTool::EMOTION {
            let emotion =
                emotion.ok_or_else(|| Error::validation("emotion tool requires an emotion"))?;
            let prompt = format!(
                "{};;{}",
                emotion.as_wire(),
                if prompt.is_empty() {
                    String::new()
                } else {
                    format!("{prompt},")
                }
            );
            (prompt, u32::from(emotion_level.as_u8()))
        } else {
            (prompt.to_owned(), defry)
        };

        let mut body = Map::new();
        body.insert("req_type".into(), Value::from(tool.as_wire()));
        body.insert("width".into(), Value::from(parsed.width));
        body.insert("height".into(), Value::from(parsed.height));
        body.insert("image".into(), Value::from(parsed.base64()));
        body.insert("prompt".into(), Value::from(prompt));
        body.insert("defry".into(), Value::from(defry));
        let content = self
            .request(
                wreq::Method::POST,
                &format!(
                    "{}{}",
                    self.inner.config.image_base_url,
                    Endpoint::Director.as_str()
                ),
                true,
                Some(Value::Object(body)),
                None,
                None,
            )
            .await?;
        first_image(content, &format!("{}.png", tool.as_wire()))
    }

    /// Upscale an image 4x with NovelAI's standalone upscaler.
    ///
    /// The endpoint is 4x only (the API exposes no scaling factor) and only
    /// the V5 line supports standalone upscaling.
    pub async fn upscale(
        &self,
        image: &[u8],
        factor: u32,
        model: Option<Model>,
    ) -> Result<NovelAIImage, Error> {
        if factor != 4 {
            return Err(Error::validation(
                "NovelAI's standalone upscaler is 4x only; factor must be 4 \
                 (the API accepts no other scaling factor)",
            ));
        }
        let upscale_model = model.unwrap_or(Model::V5);
        if !upscale_model.supports_upscale() {
            return Err(Error::validation(format!(
                "model {:?} does not support standalone upscaling; use one of: {}",
                upscale_model.as_wire(),
                crate::constants::UPSCALE_MODELS
                    .iter()
                    .map(|m| format!("{:?}", m.as_wire()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }
        let parsed = parse_image(image)?;
        let mut body = Map::new();
        body.insert("image".into(), Value::from(parsed.base64()));
        body.insert("model".into(), Value::from(upscale_model.as_wire()));
        body.insert("declared_blur_sigma".into(), Value::from(0));
        let content = self
            .request(
                wreq::Method::POST,
                &format!(
                    "{}{}",
                    self.inner.config.image_base_url,
                    Endpoint::Upscale.as_str()
                ),
                true,
                Some(Value::Object(body)),
                None,
                None,
            )
            .await?;
        first_image(content, "upscaled.png")
    }

    /// Annotate an image with a ControlNet model.
    ///
    /// `/ai/annotate-image` is the one endpoint still on the legacy host.
    pub async fn annotate(
        &self,
        image: &[u8],
        model: ControlNetModel,
    ) -> Result<NovelAIImage, Error> {
        let parsed = parse_image(image)?;
        let mut parameters = Map::new();
        parameters.insert("image".into(), Value::from(parsed.base64()));
        let mut body = Map::new();
        body.insert("model".into(), Value::from(model.as_wire()));
        body.insert("parameters".into(), Value::Object(parameters));
        let content = self
            .request(
                wreq::Method::POST,
                &format!(
                    "{}{}",
                    self.inner.config.legacy_image_base_url,
                    Endpoint::Annotate.as_str()
                ),
                true,
                Some(Value::Object(body)),
                None,
                None,
            )
            .await?;
        first_image(content, &format!("{}.png", model.as_wire()))
    }

    /// Ask NovelAI's tagger for tags completing a partial prompt.
    pub async fn suggest_tags(
        &self,
        prompt: &str,
        model: Model,
        language: &str,
    ) -> Result<Vec<Map<String, Value>>, Error> {
        let params = [
            ("model", model.as_wire()),
            ("prompt", prompt),
            ("lang", language),
        ];
        let content = self
            .request(
                wreq::Method::GET,
                &format!(
                    "{}{}",
                    self.inner.config.image_base_url,
                    Endpoint::SuggestTags.as_str()
                ),
                true,
                None,
                Some(&params),
                None,
            )
            .await?;
        let value = json_object(&content)?;
        let tags = value
            .get("tags")
            .cloned()
            .unwrap_or(Value::Array(Vec::new()));
        let Value::Array(items) = tags else {
            return Err(Error::response("NovelAI returned invalid tag suggestions"));
        };
        items
            .into_iter()
            .map(|item| match item {
                Value::Object(map) => Ok(map),
                _ => Err(Error::response("NovelAI returned invalid tag suggestions")),
            })
            .collect()
    }
    // ── account ──────────────────────────────────────────────────────────

    /// The account's subscription record.
    pub async fn get_subscription(&self) -> Result<Map<String, Value>, Error> {
        self.account_json(Endpoint::Subscription).await
    }

    /// The account's user-data record.
    pub async fn get_user_data(&self) -> Result<Map<String, Value>, Error> {
        self.account_json(Endpoint::UserData).await
    }

    async fn account_json(&self, endpoint: Endpoint) -> Result<Map<String, Value>, Error> {
        let content = self
            .request(
                wreq::Method::GET,
                &format!(
                    "{}{}",
                    self.inner.config.account_base_url,
                    endpoint.as_str()
                ),
                true,
                None,
                None,
                None,
            )
            .await?;
        let value = json_object(&content)?;
        match value {
            Value::Object(map) => Ok(map),
            _ => Err(Error::response(
                "NovelAI returned a non-object JSON response",
            )),
        }
    }
}
