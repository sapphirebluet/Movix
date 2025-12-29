use async_trait::async_trait;

use crate::streaming::{StreamError, StreamProvider};

const FILMPALAST_DOMAIN: &str = "https://filmpalast.to/stream";

pub struct FilmpalastToProvider {
    client: reqwest::Client,
}

impl FilmpalastToProvider {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        Self { client }
    }

    fn normalize_title(title: &str) -> String {
        let normalized: String = title
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_lowercase()
                } else {
                    '-'
                }
            })
            .collect();

        let mut result = String::with_capacity(normalized.len());
        let mut prev_dash = true;
        for c in normalized.chars() {
            if c == '-' {
                if !prev_dash {
                    result.push(c);
                    prev_dash = true;
                }
            } else {
                result.push(c);
                prev_dash = false;
            }
        }
        result.trim_matches('-').to_string()
    }

    fn extract_voe_url(html: &str) -> Option<String> {
        let marker = r#"href="https://voe.sx/"#;
        let start = html.find(marker)? + 6;
        let rest = &html[start..];
        let end = rest.find('"')?;
        Some(rest[..end].to_string())
    }
}

impl Default for FilmpalastToProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StreamProvider for FilmpalastToProvider {
    fn name(&self) -> &str {
        "filmpalastto"
    }

    async fn get_stream_page_url(&self, title: &str) -> Result<String, StreamError> {
        let slug = Self::normalize_title(title);
        let url = format!("{}/{}", FILMPALAST_DOMAIN, slug);

        let response = self
            .client
            .get(&url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await
            .map_err(|e| StreamError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(StreamError::NotFound(format!(
                "Page not found for title: {}",
                title
            )));
        }

        let html = response
            .text()
            .await
            .map_err(|e| StreamError::Network(e.to_string()))?;

        match Self::extract_voe_url(&html) {
            Some(voe_url) => Ok(voe_url),
            None => Err(StreamError::NotFound(
                "No VOE URL found on page".to_string(),
            )),
        }
    }
}
