use envconfig::Envconfig;

#[derive(Clone, Debug, Envconfig)]
pub struct EmbeddingConfig {
    #[envconfig(from = "EMBEDDING_SERVICE_URL")]
    pub service_url: String,
}
