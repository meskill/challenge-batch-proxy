use crate::embedding::config::EmbeddingConfig;
use crate::http::HttpClient;
use crate::types::embedding::Embedding;
use reqwest::{Error, Response};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Upstream {
    config: EmbeddingConfig,
    http_client: HttpClient,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TruncationDirection {
    Left,
    Right,
}

#[derive(Debug, Serialize)]
pub struct EmbedUpstreamRequest {
    pub inputs: String,
    pub normalize: bool,
    pub prompt_name: Option<String>,
    pub truncate: bool,
    pub truncation_direction: TruncationDirection,
}

#[derive(Deserialize)]
pub struct EmbedUpstreamResponse(pub Vec<Embedding>);

impl Upstream {
    pub fn new(config: EmbeddingConfig, http_client: HttpClient) -> Self {
        Self {
            config,
            http_client,
        }
    }

    pub async fn health_check(&self) -> Result<Response, Error> {
        let url = self.build_url("/health");
        self.execute_get(&url).await
    }

    pub async fn embed(&self, request: &EmbedUpstreamRequest) -> Result<Response, Error> {
        let url = self.build_url("/embed");
        self.execute_post(&url, request).await
    }

    // Private helper methods
    async fn execute_get(&self, url: &str) -> Result<Response, Error> {
        self.http_client.client().get(url).send().await
    }

    async fn execute_post<T: Serialize>(&self, url: &str, body: &T) -> Result<Response, Error> {
        self.http_client.client().post(url).json(body).send().await
    }

    fn build_url(&self, path: &str) -> String {
        format!("{}{}", self.config.service_url, path)
    }
}
