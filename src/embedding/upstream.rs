use crate::embedding::config::EmbeddingConfig;
use crate::embedding::{EmbedError, UpstreamErrorResponse};
use crate::http::HttpClient;
use crate::types::embedding::Embedding;
use crate::types::truncation::TruncationDirection;
use reqwest::{Error, Response};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct Upstream {
    config: EmbeddingConfig,
    http_client: HttpClient,
}

#[derive(Debug, Serialize)]
pub struct EmbedUpstreamRequest {
    pub inputs: Vec<String>,
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

    #[tracing::instrument(skip(self))]
    pub async fn embed(
        &self,
        request: &EmbedUpstreamRequest,
    ) -> Result<EmbedUpstreamResponse, EmbedError> {
        let url = self.build_url("/embed");
        let response = self.execute_post(&url, request).await?;

        // Handle upstream errors
        if !response.status().is_success() {
            let status = response.status();
            tracing::error!(%status, "Text-embedding service returned error status");

            // Try to parse error response
            match response.json::<UpstreamErrorResponse>().await {
                Ok(error_response) => {
                    return Err(EmbedError::from_upstream_response(status, error_response));
                }
                Err(_) => {
                    // If we can't parse the error response, create a generic error
                    return Err(EmbedError::Unknown {
                        status_code: status.as_u16(),
                        message: format!("Upstream service error: {}", status),
                    });
                }
            }
        }

        // Parse successful response
        let response: EmbedUpstreamResponse = response.json().await?;

        // Check if response contains embeddings
        if response.0.is_empty() {
            return Err(EmbedError::EmptyResponse);
        }

        Ok(response)
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
