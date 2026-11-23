//! Synchronous facade backed by an owned Tokio runtime.
//!
//! UniFFI's Go generator lowers `async fn` to a **blocking** call and the C++
//! generator does not support async at all, so both bindings build against
//! these signatures. Rust callers that already run inside a runtime must use
//! the async API — calling the blocking one there would deadlock, so it
//! returns [`Error::Config`] instead of panicking.

use std::sync::Arc;
use std::time::Duration;

use crate::client::{GenerationStream, NovelAiClient};
use crate::config::ClientConfig;
use crate::constants::{ControlNetModel, DirectorTool, Emotion, EmotionLevel, Model};
use crate::error::Error;
use crate::models::GenerationRequest;
use crate::response::{GenerationEvent, NovelAIImage};
use serde_json::{Map, Value};

/// A NovelAI client driving every operation on its own Tokio runtime.
#[derive(Debug, Clone)]
pub struct BlockingNovAiClient {
    client: NovelAiClient,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl BlockingNovAiClient {
    /// Build a client and its runtime.
    pub fn new(config: ClientConfig) -> Result<Self, Error> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|e| Error::config(format!("failed to start Tokio runtime: {e}")))?;
        let client = NovelAiClient::new(config)?;
        Ok(Self {
            client,
            runtime: Arc::new(runtime),
        })
    }

    /// The async client, for callers that need finer control.
    pub fn inner(&self) -> &NovelAiClient {
        &self.client
    }

    fn block_on<F>(&self, future: F) -> Result<F::Output, Error>
    where
        F: std::future::Future,
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(Error::config(
                "blocking API called from within an async runtime; use the async client",
            ));
        }
        Ok(self.runtime.block_on(future))
    }

    /// Generate images.
    pub fn generate(&self, request: &GenerationRequest) -> Result<Vec<NovelAIImage>, Error> {
        self.block_on(self.client.generate(request))?
    }

    /// Collect every streaming event (buffered streaming for C++ and simple Go
    /// callers).
    pub fn collect_generation(
        &self,
        request: &GenerationRequest,
        max_events: Option<u32>,
    ) -> Result<Vec<GenerationEvent>, Error> {
        self.block_on(async {
            let mut stream = self.client.stream_generation(request).await?;
            let mut events = Vec::new();
            while let Some(event) = futures_util::StreamExt::next(&mut stream).await {
                events.push(event?);
                if max_events.is_some_and(|max| events.len() >= max as usize) {
                    break;
                }
            }
            Ok(events)
        })?
    }

    /// Pull-based streaming handle; `next()` drives one event at a time.
    pub fn stream_generation(&self, request: &GenerationRequest) -> Result<BlockingEventStream, Error> {
        let stream = self.block_on(self.client.stream_generation(request))??;
        Ok(BlockingEventStream {
            stream,
            runtime: Arc::clone(&self.runtime),
        })
    }

    /// Encode a vibe reference.
    pub fn encode_vibe(
        &self,
        reference: &str,
        information_extracted: f64,
        model: Model,
    ) -> Result<String, Error> {
        self.block_on(self.client.encode_vibe(reference, information_extracted, model))?
    }

    /// Run a Director tool.
    pub fn director(
        &self,
        tool: DirectorTool,
        image: &[u8],
        prompt: &str,
        defry: u32,
        emotion: Option<Emotion>,
        emotion_level: EmotionLevel,
    ) -> Result<NovelAIImage, Error> {
        self.block_on(
            self.client
                .director(tool, image, prompt, defry, emotion, emotion_level),
        )?
    }

    /// Upscale an image.
    pub fn upscale(
        &self,
        image: &[u8],
        factor: u32,
        model: Option<Model>,
    ) -> Result<NovelAIImage, Error> {
        self.block_on(self.client.upscale(image, factor, model))?
    }

    /// Annotate an image.
    pub fn annotate(&self, image: &[u8], model: ControlNetModel) -> Result<NovelAIImage, Error> {
        self.block_on(self.client.annotate(image, model))?
    }

    /// Suggest tags.
    pub fn suggest_tags(
        &self,
        prompt: &str,
        model: Model,
        language: &str,
    ) -> Result<Vec<Map<String, Value>>, Error> {
        self.block_on(self.client.suggest_tags(prompt, model, language))?
    }

    /// The subscription record.
    pub fn get_subscription(&self) -> Result<Map<String, Value>, Error> {
        self.block_on(self.client.get_subscription())?
    }

    /// The user-data record.
    pub fn get_user_data(&self) -> Result<Map<String, Value>, Error> {
        self.block_on(self.client.get_user_data())?
    }

    /// The bearer token (logging in if needed).
    pub fn access_token(&self) -> Result<String, Error> {
        self.block_on(self.client.access_token())?
    }

    /// The per-request timeout used for unary calls.
    pub fn timeout(&self) -> Duration {
        self.client.config().timeout
    }
}

/// A pull-based stream that blocks for one event at a time.
pub struct BlockingEventStream {
    stream: GenerationStream,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl BlockingEventStream {
    /// The next event, or `None` at end of stream.
    #[allow(clippy::should_implement_trait)] // pull-based, not an Iterator
    pub fn next(&mut self) -> Result<Option<GenerationEvent>, Error> {
        let runtime = Arc::clone(&self.runtime);
        match runtime.block_on(futures_util::StreamExt::next(&mut self.stream)) {
            None => Ok(None),
            Some(event) => event.map(Some),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::Credentials;

    #[test]
    fn rejects_use_from_an_async_runtime() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let client = BlockingNovAiClient::new(ClientConfig::new(Credentials::token("t")))
            .expect("client");
        runtime.block_on(async {
            let err = client.get_user_data().unwrap_err();
            assert!(err.to_string().contains("within an async runtime"));
        });
    }
}
