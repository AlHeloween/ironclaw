//! Browser-based content fetch using Playwright (real Chromium).
//! Handles JavaScript-rendered pages and some Cloudflare-protected sites.

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserRequest {
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct BrowserResponse {
    pub url: String,
    pub title: String,
    pub content: String,
    pub content_length: usize,
}

pub async fn execute_browser_fetch(
    request: &BrowserRequest,
) -> Result<BrowserResponse, String> {
    let url = &request.url;
    validate_browser_url(url)?;

    tracing::info!("Launching Playwright browser for: {}", url);

    let playwright = playwright_rs::Playwright::launch()
        .await
        .map_err(|e| format!("Playwright driver failed: {}", e))?;

    let browser = playwright
        .chromium()
        .launch()
        .await
        .map_err(|e| format!("Browser launch failed: {}", e))?;

    let page = browser
        .new_page()
        .await
        .map_err(|e| format!("Page creation failed: {}", e))?;

    // Navigate
    let _response = page
        .goto(url, None)
        .await
        .map_err(|e| format!("Navigation failed: {}", e))?;

    // Wait for page to load (JS execution, Cloudflare challenge resolution)
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    let title = page.title().await.unwrap_or_default();
    let content = page
        .content()
        .await
        .map_err(|e| format!("Content extraction failed: {}", e))?;

    let content_length = content.len();

    let _ = browser.close().await;

    Ok(BrowserResponse {
        url: url.clone(),
        title,
        content,
        content_length,
    })
}

fn validate_browser_url(url: &str) -> Result<(), String> {
    if url.len() > 2048 {
        return Err("URL exceeds maximum length of 2048 characters".to_string());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    Ok(())
}
