mod shared;

use shared::{EmbeddingMockServer, TestApp};
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

#[tokio::test]
async fn test_health_endpoint() {
    let mock_server = EmbeddingMockServer::start().await;
    let test_app = TestApp::new(&mock_server);

    let response = test_app.server.get("/health").await;

    response.assert_status_ok();

    let body: serde_json::Value = response.json();

    insta::assert_json_snapshot!(body, @r#"
    {
      "status": "ok"
    }
    "#);
}

#[tokio::test]
async fn test_ready_endpoint() {
    let mock_server = EmbeddingMockServer::start().await;
    let test_app = TestApp::new(&mock_server);

    let response = test_app.server.get("/ready").await;

    response.assert_status_service_unavailable();

    let body: serde_json::Value = response.json();

    insta::assert_json_snapshot!(body, @r#"
    {
      "details": "embedding service unhealthy",
      "status": "not_ready"
    }
    "#);

    {
        let _mock = Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount_as_scoped(&mock_server.server)
            .await;

        let response = test_app.server.get("/ready").await;

        response.assert_status_ok();
    }

    let response = test_app.server.get("/ready").await;

    response.assert_status_service_unavailable();

    let body: serde_json::Value = response.json();

    insta::assert_json_snapshot!(body, @r#"
    {
      "details": "embedding service unhealthy",
      "status": "not_ready"
    }
    "#);
}
