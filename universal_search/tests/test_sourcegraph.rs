//! Integration tests for Sourcegraph API with mocked HTTP.

use universal_search_service::config::SourcegraphConfig;
use universal_search_service::web::{execute_sourcegraph, SourcegraphRequest};

#[tokio::test]
async fn test_sourcegraph_search_success() {
    let client = reqwest::Client::new();
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r##"{"data":{"search":{"results":{"results":[{"type":"file","file":{"path":"src/main.rs","name":"main.rs"},"repository":{"name":"owner/repo"},"line":42}],"matchCount":1}}}}"##)
        .create();

    let config = SourcegraphConfig {
        api_url: mock_url,
        access_token: None,
    };

    let request = SourcegraphRequest {
        query: "async fn search".to_string(),
        url: None,
        count: None,
    };

    let response = execute_sourcegraph(&client, &config, &request).await.unwrap();
    assert_eq!(response.query, "async fn search");
    assert_eq!(response.match_count, 1);
    assert_eq!(response.results.len(), 1);
    let result = &response.results[0];
    assert_eq!(result.repository.as_ref().unwrap(), "owner/repo");
    assert_eq!(result.path.as_ref().unwrap(), "src/main.rs");
    assert_eq!(result.line.as_ref().unwrap(), &42);
    assert_eq!(result.source, "sourcegraph");
    mock.assert();
}

#[tokio::test]
async fn test_sourcegraph_with_repo_filter() {
    let client = reqwest::Client::new();
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/")
        .match_body(mockito::Matcher::Regex("repo:myorg/myrepo".to_string()))
        .with_status(200)
        .with_body(r##"{"data":{"search":{"results":{"results":[],"matchCount":0}}}}"##)
        .create();

    let config = SourcegraphConfig {
        api_url: mock_url,
        access_token: None,
    };

    let request = SourcegraphRequest {
        query: "test".to_string(),
        url: Some("myorg/myrepo".to_string()),
        count: None,
    };

    let response = execute_sourcegraph(&client, &config, &request).await.unwrap();
    assert_eq!(response.match_count, 0);
    mock.assert();
}

#[tokio::test]
async fn test_sourcegraph_with_token() {
    let client = reqwest::Client::new();
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/")
        .match_header("Authorization", "token sg_test_token")
        .with_status(200)
        .with_body(r##"{"data":{"search":{"results":{"results":[],"matchCount":0}}}}"##)
        .create();

    let config = SourcegraphConfig {
        api_url: mock_url,
        access_token: Some("sg_test_token".to_string()),
    };

    let request = SourcegraphRequest {
        query: "test".to_string(),
        url: None,
        count: None,
    };

    let response = execute_sourcegraph(&client, &config, &request).await.unwrap();
    assert_eq!(response.match_count, 0);
    mock.assert();
}

#[tokio::test]
async fn test_sourcegraph_empty_results() {
    let client = reqwest::Client::new();
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/")
        .with_status(200)
        .with_body(r##"{"data":{"search":{"results":{"results":[],"matchCount":0}}}}"##)
        .create();

    let config = SourcegraphConfig {
        api_url: mock_url,
        access_token: None,
    };

    let request = SourcegraphRequest {
        query: "nonexistent_xyz_123".to_string(),
        url: None,
        count: None,
    };

    let response = execute_sourcegraph(&client, &config, &request).await.unwrap();
    assert_eq!(response.match_count, 0);
    assert!(response.results.is_empty());
    mock.assert();
}

#[tokio::test]
async fn test_sourcegraph_error_response() {
    let client = reqwest::Client::new();
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/")
        .with_status(200)
        .with_body(r##"{"errors":[{"message":"Rate limit exceeded"}]}"##)
        .create();

    let config = SourcegraphConfig {
        api_url: mock_url,
        access_token: None,
    };

    let request = SourcegraphRequest {
        query: "test".to_string(),
        url: None,
        count: None,
    };

    let result = execute_sourcegraph(&client, &config, &request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Rate limit exceeded"));
    mock.assert();
}
