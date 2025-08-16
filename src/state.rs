use crate::config::AppConfig;
use crate::embedding::EmbeddingState;

#[derive(Clone)]
pub struct AppState {
    pub embedding: EmbeddingState,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let embedding = EmbeddingState::new(config.embedding);

        Self { embedding }
    }
}
