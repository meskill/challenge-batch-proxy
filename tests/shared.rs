use axum_test::TestServer;
use challenge_batch_proxy::http::app_router;
use challenge_batch_proxy::state::AppState;
use challenge_batch_proxy::{AppConfig, EmbeddingConfig};
use wiremock::MockServer as WireMockServer;

pub const TEST_DEFAULT_BATCH_DURATION_MS: u64 = 20;
pub const TEST_DEFAULT_BATCH_SIZE: usize = 4;

pub struct EmbeddingMockServer {
    pub server: WireMockServer,
}

impl EmbeddingMockServer {
    pub async fn start() -> Self {
        let server = WireMockServer::start().await;

        Self { server }
    }
}

pub struct TestApp {
    pub server: TestServer,
}

impl TestApp {
    pub fn new(embedding_mock_server: &EmbeddingMockServer) -> Self {
        Self::with_config(app_config_batch(embedding_mock_server))
    }

    pub fn with_config(config: AppConfig) -> Self {
        let state = AppState::new(config);
        let app = app_router(state);
        let server = TestServer::new(app).unwrap();

        Self { server }
    }
}

// Alternative approach using direct struct construction
pub fn app_config_batch(embedding_mock_server: &EmbeddingMockServer) -> AppConfig {
    AppConfig {
        bind_host: "127.0.0.1:0".to_string(),
        embedding: EmbeddingConfig {
            service_url: embedding_mock_server.server.uri(),
            batch_duration_ms: TEST_DEFAULT_BATCH_DURATION_MS,
            batch_size: TEST_DEFAULT_BATCH_SIZE,
            use_batch: true,
        },
    }
}

#[allow(unused)] // it's actually used by tests
pub fn app_config_no_batch(embedding_mock_server: &EmbeddingMockServer) -> AppConfig {
    AppConfig {
        bind_host: "127.0.0.1:0".to_string(),
        embedding: EmbeddingConfig {
            service_url: embedding_mock_server.server.uri(),
            batch_duration_ms: 0,
            batch_size: 1,
            use_batch: false,
        },
    }
}
