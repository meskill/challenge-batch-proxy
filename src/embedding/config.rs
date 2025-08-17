use envconfig::Envconfig;

#[derive(Clone, Debug, Envconfig)]
pub struct EmbeddingConfig {
    #[envconfig(from = "EMBEDDING_SERVICE_URL")]
    pub service_url: String,

    #[envconfig(from = "EMBEDDING_BATCH_DURATION_MS", default = "200")]
    pub batch_duration_ms: u64,

    #[envconfig(from = "EMBEDDING_BATCH_SIZE", default = "20")]
    pub batch_size: usize,

    #[envconfig(from = "EMBEDDING_USE_BATCH", default = "true")]
    pub use_batch: bool,
}
