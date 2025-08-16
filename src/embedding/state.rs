use crate::embedding::config::EmbeddingConfig;
use crate::embedding::upstream::Upstream;
use crate::http::HttpClient;

#[derive(Clone)]
pub struct EmbeddingState {
    pub upstream: Upstream,
}

impl EmbeddingState {
    pub fn new(config: EmbeddingConfig) -> Self {
        let http_client = HttpClient::new();
        let upstream = Upstream::new(config.clone(), http_client.clone());

        Self { upstream }
    }
}
