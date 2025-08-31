use std::time::Duration;

use batch::GroupBatcher;

use crate::embedding::batch::EmbeddingBatch;
use crate::embedding::config::EmbeddingConfig;
use crate::embedding::upstream::Upstream;
use crate::embedding::{EmbedError, EmbedRequest, EmbedResponse, EmbedUpstreamRequest, OneOrMany};
use crate::http::HttpClient;

#[derive(Clone)]
pub struct EmbeddingState {
    pub upstream: Upstream,
    batcher: GroupBatcher<EmbeddingBatch>,
    use_batch: bool,
}

impl EmbeddingState {
    pub fn new(config: EmbeddingConfig) -> Self {
        let http_client = HttpClient::new();
        let upstream = Upstream::new(config.clone(), http_client.clone());
        let batcher = GroupBatcher::new(
            Duration::from_millis(config.batch_duration_ms),
            config.batch_size,
            EmbeddingBatch::new(upstream.clone()),
        );

        Self {
            upstream,
            batcher,
            use_batch: config.use_batch,
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn embed(&self, request: EmbedRequest) -> Result<EmbedResponse, EmbedError> {
        if self.use_batch {
            let embedding = self.batcher.run(request).await?;

            Ok(EmbedResponse::from(embedding))
        } else {
            let upstream_request = EmbedUpstreamRequest {
                inputs: request.input.into_vec(),
                normalize: request.normalize,
                prompt_name: request.prompt_name,
                truncate: request.truncate,
                truncation_direction: request.truncation_direction,
            };

            let response = self.upstream.embed(&upstream_request).await?;

            Ok(EmbedResponse::from(OneOrMany::from(response.0)))
        }
    }
}
