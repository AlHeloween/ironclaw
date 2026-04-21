//! Integration tests for web search (Firecrawl API) with mocked HTTP.

use universal_search_service::config::FirecrawlConfig;
use universal_search_service::web::{execute_web_search, WebSearchRequest, execute_context, ContextRequest};

#[tokio::test]
async fn test_web_search_success() {
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/v1/search")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r##"{"success":true,"data":{"results":[{"url":"http://example.com/page1","title":"Test Page","description":"A test page","markdown":"# Test\nContent here"}]}}"##)
        .create();

    let config = FirecrawlConfig {
        api_url: mock_url,
        ..Default::default()
    };

    let request = WebSearchRequest {
        query: "test query".to_string(),
        count: Some(5),
        scrape_formats: None,
        only_main_content: None,
    };

    let response = execute_web_search(&config, &request).await.unwrap();
    assert_eq!(response.query, "test query");
    assert_eq!(response.result_count, 1);
    assert_eq!(response.results[0].title, "Test Page");
    assert_eq!(response.results[0].url, "http://example.com/page1");
    mock.assert();
}

#[tokio::test]
async fn test_web_search_with_api_key() {
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/v1/search")
        .match_header("Authorization", "Bearer test_key_123")
        .with_status(200)
        .with_body(r##"{"success":true,"data":{"results":[]}}"##)
        .create();

    let config = FirecrawlConfig {
        api_url: mock_url,
        api_key: Some("test_key_123".to_string()),
        ..Default::default()
    };

    let request = WebSearchRequest {
        query: "test".to_string(),
        count: None,
        scrape_formats: None,
        only_main_content: None,
    };

    let response = execute_web_search(&config, &request).await.unwrap();
    assert_eq!(response.result_count, 0);
    mock.assert();
}

#[tokio::test]
async fn test_web_search_error_4xx() {
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/v1/search")
        .with_status(401)
        .with_body(r##"{"error":"Unauthorized"}"##)
        .create();

    let config = FirecrawlConfig {
        api_url: mock_url,
        ..Default::default()
    };

    let request = WebSearchRequest {
        query: "test".to_string(),
        count: None,
        scrape_formats: None,
        only_main_content: None,
    };

    let result = execute_web_search(&config, &request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("401"));
    mock.assert();
}

#[tokio::test]
async fn test_context_scrape_success() {
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/v1/scrape")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r##"{"success":true,"data":{"markdown":"# Article Title\nThis is the article content.","title":"Article Title","description":"A great article","sourceURL":"http://example.com/article"}}"##)
        .create();

    let config = FirecrawlConfig {
        api_url: mock_url,
        ..Default::default()
    };

    let request = ContextRequest {
        query: "summarize".to_string(),
        url: "http://example.com/article".to_string(),
        scrape_formats: None,
        only_main_content: None,
    };

    let response = execute_context(&config, &request).await.unwrap();
    assert_eq!(response.query, "summarize");
    assert_eq!(response.url, "http://example.com/article");
    assert!(response.markdown.unwrap().contains("Article Title"));
    mock.assert();
}

#[tokio::test]
async fn test_context_scrape_url_too_long() {
    let config = FirecrawlConfig::default();
    let request = ContextRequest {
        query: "test".to_string(),
        url: "http://example.com/".to_string() + &"a".repeat(3000),
        scrape_formats: None,
        only_main_content: None,
    };

    let result = execute_context(&config, &request).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("exceeds maximum length"));
}

#[tokio::test]
async fn test_context_scrape_error_404() {
    let mut mock_server = mockito::Server::new_async().await;
    let mock_url = mock_server.url();

    let mock = mock_server.mock("POST", "/v1/scrape")
        .with_status(404)
        .with_body(r##"{"error":"Not Found"}"##)
        .create();

    let config = FirecrawlConfig {
        api_url: mock_url,
        ..Default::default()
    };

    let request = ContextRequest {
        query: "test".to_string(),
        url: "http://example.com/missing".to_string(),
        scrape_formats: None,
        only_main_content: None,
    };

    let result = execute_context(&config, &request).await;
    assert!(result.is_err());
    mock.assert();
}
