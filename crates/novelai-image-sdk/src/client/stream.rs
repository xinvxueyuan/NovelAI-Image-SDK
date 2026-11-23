//! Streaming generation: MessagePack events as the driver produces them.
//!
//! Part of [`super`]'s `NovelAiClient`.

use futures_util::StreamExt;

use crate::constants::Endpoint;
use crate::error::Error;
use crate::models::GenerationRequest;
use crate::payload::build_generation_payload;
use crate::response::{MessagePackStreamParser, check_status};

use super::{GenerationStream, transport_error};

impl super::NovelAiClient {
    /// Stream V4 / V4.5 / V5 events as the driver produces them.
    ///
    /// V3 models are rejected: only V4+ speaks MessagePack. Dropping the
    /// stream cancels the request.
    pub async fn stream_generation(
        &self,
        request: &GenerationRequest,
    ) -> Result<GenerationStream, Error> {
        let request = request.clone().normalize()?;
        let prepared = self.prepare_references(request).await?;
        if !prepared.model.is_v4() {
            return Err(Error::validation(
                "real-time generation requires a V4+ / V5 model",
            ));
        }
        let url = format!(
            "{}{}",
            self.inner.config.image_base_url,
            Endpoint::ImageStream.as_str()
        );
        let token = self.access_token().await?;
        let request_builder = self
            .base_request_with(
                wreq::Method::POST,
                &url,
                Some(&token),
                &[("Accept", "application/x-msgpack")],
            )
            .json(&build_generation_payload(&prepared));
        let timeout = self.inner.config.timeout;

        // `wreq`'s client-wide timeout is a *total* deadline and would kill
        // long generations; apply an idle timeout per chunk read instead.
        let response = tokio::time::timeout(timeout, request_builder.send())
            .await
            .map_err(|_| Error::Timeout {
                message: "NovelAI streaming request timed out".into(),
            })?
            .map_err(transport_error)?;
        let status = response.status().as_u16();
        let mut body = response.bytes_stream();
        if status >= 400 {
            check_status(status, &super::drain_body(&mut body).await?)?;
        }
        let stream = async_stream::try_stream! {
            let mut parser = MessagePackStreamParser::new();
            loop {
                let chunk = match tokio::time::timeout(timeout, body.next()).await {
                    Err(_) => {
                        Err(Error::Timeout {
                            message: "NovelAI streaming request timed out".into(),
                        })?
                    }
                    Ok(None) => break,
                    Ok(Some(chunk)) => chunk.map_err(transport_error)?,
                };
                for event in parser.feed(&chunk)? {
                    yield event;
                }
            }
            parser.finish()?;
        };
        Ok(stream.boxed())
    }
}
