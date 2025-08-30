mod shared;

use challenge_batch_proxy::embedding::EmbedRequest;
use reqwest::StatusCode;
use serde_json::json;
use shared::TEST_DEFAULT_BATCH_DURATION_MS;
use shared::{EmbeddingMockServer, TestApp};
use std::time::Instant;
use wiremock::matchers::{method, path};
use wiremock::{Mock, ResponseTemplate};

macro_rules! assert_elapsed_time_difference {
    ($start:expr) => {
        assert!(
            $start.elapsed().as_millis() < 5,
            "Requests were executed for too long"
        );
    };
    ($start:expr, $batch_duration_ms:expr) => {
        assert!(
            ($start.elapsed().as_millis() as u64).abs_diff($batch_duration_ms) < 5,
            "Requests were not batched"
        );
    };
}

mod validation {
    use super::*;

    #[tokio::test]
    async fn test_invalid_empty_json() {
        let mock_server = EmbeddingMockServer::start().await;
        let test_app = TestApp::new(&mock_server);

        let payload = json!({});

        let response = test_app.server.post("/embed").json(&payload).await;

        response.assert_status_unprocessable_entity();

        let body: serde_json::Value = response.json();

        insta::assert_json_snapshot!(body, @r#"
        {
          "message": "Failed to deserialize the JSON body into the target type: missing field `input` at line 1 column 2",
          "name": "JsonRejection",
          "status": 422
        }
        "#);
    }

    #[tokio::test]
    async fn test_invalid_truncation_direction() {
        let mock_server = EmbeddingMockServer::start().await;
        let test_app = TestApp::new(&mock_server);

        let payload = json!({
            "input": "test",
            "truncation_direction": "outside"
        });

        let response = test_app.server.post("/embed").json(&payload).await;

        response.assert_status_unprocessable_entity();

        let body: serde_json::Value = response.json();

        insta::assert_json_snapshot!(body, @r#"
        {
          "message": "Failed to deserialize the JSON body into the target type: truncation_direction: unknown variant `outside`, expected `Left` or `Right` at line 1 column 48",
          "name": "JsonRejection",
          "status": 422
        }
        "#);
    }
}

mod upstream_error {
    use super::*;

    #[tokio::test]
    async fn test_embed_endpoint_upstream_error() {
        let mock_server = EmbeddingMockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "error": "Model is overloaded",
                "error_type": "Overload"
            })))
            .mount(&mock_server.server)
            .await;

        let test_app = TestApp::new(&mock_server);

        let payload = EmbedRequest::new("Invalid input that causes error");

        let response = test_app.server.post("/embed").json(&payload).await;

        response.assert_status(StatusCode::TOO_MANY_REQUESTS);

        let body: serde_json::Value = response.json();

        insta::assert_json_snapshot!(body, @r#"
        {
          "message": "Model is overloaded: Model is overloaded",
          "name": "Overloaded",
          "status": 429
        }
        "#);
    }

    #[tokio::test]
    async fn test_embed_endpoint_unknown_upstream_error() {
        let mock_server = EmbeddingMockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .respond_with(ResponseTemplate::new(400).set_body_json(json!({
                "error": "Invalid input",
                "error_type": "validation"
            })))
            .mount(&mock_server.server)
            .await;

        let test_app = TestApp::new(&mock_server);

        let payload = EmbedRequest::new("Invalid input that causes error");

        let response = test_app.server.post("/embed").json(&payload).await;

        response.assert_status(StatusCode::BAD_GATEWAY);

        let body: serde_json::Value = response.json();

        insta::assert_json_snapshot!(body, @r#"
        {
          "message": "Unknown upstream error (status 400): Invalid input",
          "name": "Unknown",
          "status": 502
        }
        "#);
    }
}

mod batch_enabled {
    use challenge_batch_proxy::types::truncation::TruncationDirection;
    use wiremock::matchers::body_partial_json;

    use super::shared::app_config_batch as app_config;
    use super::*;

    #[tokio::test]
    async fn test_single_request() {
        let mock_server = EmbeddingMockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([[0.1, 0.2, 0.3]])))
            .mount(&mock_server.server)
            .await;

        let test_app = TestApp::with_config(app_config(&mock_server));

        let payload = EmbedRequest::new("Hello, world!");

        let response = test_app.server.post("/embed").json(&payload).await;

        response.assert_status_ok();

        let body: Vec<f32> = response.json();
        assert_eq!(body, vec![0.1, 0.2, 0.3]);
    }

    #[tokio::test]
    async fn test_multiple_request_batched_by_time() {
        let mock_server = EmbeddingMockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                [0.1, 0.2, 0.3],
                [0.4, 0.5, 0.6],
                [0.7, 0.8, 0.9],
            ])))
            .mount(&mock_server.server)
            .await;

        let test_app = TestApp::with_config(app_config(&mock_server));

        let start_time = Instant::now();

        let (first, second, third) = tokio::join!(
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test1")),
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test2")),
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test3")),
        );

        // the execution time should be very close for batching duration
        assert_elapsed_time_difference!(start_time, TEST_DEFAULT_BATCH_DURATION_MS);

        first.assert_status_ok();
        second.assert_status_ok();
        third.assert_status_ok();

        let body1: Vec<f32> = first.json();
        let body2: Vec<f32> = second.json();
        let body3: Vec<f32> = third.json();

        assert_eq!(body1, vec![0.1, 0.2, 0.3]);
        assert_eq!(body2, vec![0.4, 0.5, 0.6]);
        assert_eq!(body3, vec![0.7, 0.8, 0.9]);
    }

    #[tokio::test]
    async fn test_multiple_request_batched_by_size() {
        let mock_server = EmbeddingMockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                [0.1, 0.2, 0.3],
                [0.4, 0.5, 0.6],
                [0.7, 0.8, 0.9],
                [0.3, 0.5, 0.7]
            ])))
            .mount(&mock_server.server)
            .await;

        let test_app = TestApp::with_config(app_config(&mock_server));

        let start_time = Instant::now();

        let (first, second, third, fourth) = tokio::join!(
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test1")),
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test2")),
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test3")),
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test4")),
        );

        // reaching the batch size should trigger request immediately
        assert_elapsed_time_difference!(start_time);

        first.assert_status_ok();
        second.assert_status_ok();
        third.assert_status_ok();
        fourth.assert_status_ok();

        let body1: Vec<f32> = first.json();
        let body2: Vec<f32> = second.json();
        let body3: Vec<f32> = third.json();
        let body4: Vec<f32> = fourth.json();

        assert_eq!(body1, vec![0.1, 0.2, 0.3]);
        assert_eq!(body2, vec![0.4, 0.5, 0.6]);
        assert_eq!(body3, vec![0.7, 0.8, 0.9]);
        assert_eq!(body4, vec![0.3, 0.5, 0.7]);
    }

    #[tokio::test]
    async fn test_multiple_request_batched_separately_by_groups() {
        let mock_server = EmbeddingMockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .and(body_partial_json(json!({
                "truncation_direction": "Left",
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!([[0.1, 0.2, 0.3], [0.7, 0.8, 0.9],])),
            )
            .mount(&mock_server.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .and(body_partial_json(json!({
                "truncation_direction": "Right",
            })))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!([[0.4, 0.5, 0.6], [0.3, 0.5, 0.7],])),
            )
            .mount(&mock_server.server)
            .await;

        let test_app = TestApp::with_config(app_config(&mock_server));

        let start_time = Instant::now();

        let to_left = |mut req: EmbedRequest| {
            req.truncation_direction = TruncationDirection::Left;
            req
        };
        let to_right = |mut req: EmbedRequest| {
            req.truncation_direction = TruncationDirection::Right;
            req
        };

        let (first, second, third, fourth) = tokio::join!(
            test_app
                .server
                .post("/embed")
                .json(&to_left(EmbedRequest::new("test1"))),
            test_app
                .server
                .post("/embed")
                .json(&to_right(EmbedRequest::new("test2"))),
            test_app
                .server
                .post("/embed")
                .json(&to_left(EmbedRequest::new("test3"))),
            test_app
                .server
                .post("/embed")
                .json(&to_right(EmbedRequest::new("test4"))),
        );

        // reaching the batch size should trigger request immediately
        assert_elapsed_time_difference!(start_time, TEST_DEFAULT_BATCH_DURATION_MS);

        first.assert_status_ok();
        second.assert_status_ok();
        third.assert_status_ok();
        fourth.assert_status_ok();

        let body1: Vec<f32> = first.json();
        let body2: Vec<f32> = second.json();
        let body3: Vec<f32> = third.json();
        let body4: Vec<f32> = fourth.json();

        assert_eq!(body1, vec![0.1, 0.2, 0.3]);
        assert_eq!(body2, vec![0.4, 0.5, 0.6]);
        assert_eq!(body3, vec![0.7, 0.8, 0.9]);
        assert_eq!(body4, vec![0.3, 0.5, 0.7]);
    }
}

mod batch_disabled {

    use wiremock::matchers::body_partial_json;

    use super::shared::app_config_no_batch as app_config;
    use super::*;

    #[tokio::test]
    async fn test_requests_not_grouped() {
        let mock_server = EmbeddingMockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .and(body_partial_json(json!({
                "inputs": ["test1"],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([[0.1, 0.2, 0.3]])))
            .expect(1)
            .mount(&mock_server.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .and(body_partial_json(json!({
                "inputs": ["test2"],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([[0.4, 0.5, 0.6]])))
            .expect(1)
            .mount(&mock_server.server)
            .await;

        Mock::given(method("POST"))
            .and(path("/embed"))
            .and(body_partial_json(json!({
                "inputs": ["test3"],
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!([[0.7, 0.8, 0.9]])))
            .expect(1)
            .mount(&mock_server.server)
            .await;

        let test_app = TestApp::with_config(app_config(&mock_server));

        let (first, second, third) = tokio::join!(
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test1")),
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test2")),
            test_app
                .server
                .post("/embed")
                .json(&EmbedRequest::new("test3")),
        );

        first.assert_status_ok();
        second.assert_status_ok();
        third.assert_status_ok();

        let body1: Vec<f32> = first.json();
        let body2: Vec<f32> = second.json();
        let body3: Vec<f32> = third.json();

        assert_eq!(body1, vec![0.1, 0.2, 0.3]);
        assert_eq!(body2, vec![0.4, 0.5, 0.6]);
        assert_eq!(body3, vec![0.7, 0.8, 0.9]);
    }
}
