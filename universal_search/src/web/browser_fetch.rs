//! Browser-based content fetch using Playwright connected to persistent Chromium via CDP.
//! Chromium must be running with --remote-debugging-port=9222 (NSSM service).

use serde::{Deserialize, Serialize};

/// CDP endpoint for the persistent Chromium service.
const CDP_URL: &str = "http://127.0.0.1:9222";

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

    tracing::info!("Connecting to persistent Chromium via CDP: {}", CDP_URL);

    let playwright = playwright_rs::Playwright::launch()
        .await
        .map_err(|e| format!("Playwright driver failed: {}", e))?;

    let browser: playwright_rs::Browser = playwright
        .chromium()
        .connect_over_cdp(CDP_URL, None::<playwright_rs::ConnectOverCdpOptions>)
        .await
        .map_err(|e| format!("CDP connect failed: {}", e))?;

    let page: playwright_rs::Page = browser
        .new_page()
        .await
        .map_err(|e| format!("Page creation failed: {}", e))?;

    // Navigate
    let _response = page
        .goto(url, None::<playwright_rs::GotoOptions>)
        .await
        .map_err(|e| format!("Navigation failed: {}", e))?;

    // Detect Cloudflare / anti-bot challenge page and wait for it to resolve
    if is_challenge_page(&page).await {
        tracing::info!("Detected anti-bot challenge page, waiting for resolution...");
        let waited = wait_for_challenge_resolve(&page, url).await;
        if !waited {
            tracing::warn!("Challenge did not resolve within timeout, returning whatever content is present");
        }
    }

    let title: String = page.title().await.unwrap_or_default();
    let content: String = page
        .content()
        .await
        .map_err(|e| format!("Content extraction failed: {}", e))?;

    let content_length = content.len();

    // Drop page reference — Chromium stays running as a service
    drop(page);

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

/// Check if the current page is a Cloudflare / anti-bot challenge.
async fn is_challenge_page(page: &playwright_rs::Page) -> bool {
    let title: String = page.title().await.unwrap_or_default();
    if title.contains("Just a moment") {
        return true;
    }
    if let Ok(content) = page.content().await {
        let c: String = content;
        if c.contains("Performing security verification")
            || c.contains("cf-turnstile")
            || c.contains("challenge-platform")
        {
            return true;
        }
    }
    false
}

/// Wait for a challenge page to resolve. Returns true if resolution detected,
/// false if we timed out.
async fn wait_for_challenge_resolve(
    page: &playwright_rs::Page,
    expected_url: &str,
) -> bool {
    let poll_interval = std::time::Duration::from_secs(2);
    let max_wait = std::time::Duration::from_secs(30);
    let start = std::time::Instant::now();

    while start.elapsed() < max_wait {
        tokio::time::sleep(poll_interval).await;

        // Check if URL changed away from challenge
        let current_url: String = page.url();
        if current_url == expected_url && !is_challenge_page(page).await {
            tracing::info!("Challenge resolved — page now shows real content");
            return true;
        }

        // Check if title changed from "Just a moment..."
        let title: String = page.title().await.unwrap_or_default();
        if !title.is_empty() && !title.contains("Just a moment") {
            // Double-check it's not still a challenge
            if !is_challenge_page(page).await {
                tracing::info!("Challenge resolved — title: {}", title);
                return true;
            }
        }

        tracing::debug!(
            "Waiting for challenge... elapsed={}s title={}",
            start.elapsed().as_secs(),
            title
        );
    }

    false
}
