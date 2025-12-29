use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use regex::Regex;
use serde_json::Value;

use crate::streaming::{StreamError, StreamResolver};

const MARKERS: &[&str] = &["@#", "^^", "~@", "%?", "*~", "!!", "#&"];
const BAIT_PATTERNS: &[&str] = &["bigbuckbunny", "test-videos.co.uk", "sample-videos.com"];

pub struct VoeResolver {
    client: reqwest::Client,
    max_redirects: usize,
}

impl VoeResolver {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/120.0.0.0")
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_default();

        Self {
            client,
            max_redirects: 5,
        }
    }

    fn rot13(text: &str) -> String {
        text.chars()
            .map(|c| {
                let code = c as u32;
                if (65..=90).contains(&code) {
                    char::from_u32(((code - 65 + 13) % 26) + 65).unwrap_or(c)
                } else if (97..=122).contains(&code) {
                    char::from_u32(((code - 97 + 13) % 26) + 97).unwrap_or(c)
                } else {
                    c
                }
            })
            .collect()
    }

    fn strip_markers(text: &str) -> String {
        MARKERS
            .iter()
            .fold(text.to_string(), |acc, m| acc.replace(m, ""))
    }

    fn shift_chars(text: &str, offset: i32) -> String {
        text.chars()
            .filter_map(|c| {
                let new_code = (c as i32) - offset;
                if new_code >= 0 {
                    char::from_u32(new_code as u32)
                } else {
                    Some(c)
                }
            })
            .collect()
    }

    fn safe_b64_decode(encoded: &str) -> Option<String> {
        let clean: String = encoded
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '+' || *c == '/' || *c == '=')
            .collect();

        let mut padded = clean;
        let padding = padded.len() % 4;
        if padding > 0 {
            padded.push_str(&"=".repeat(4 - padding));
        }

        let bytes = STANDARD.decode(&padded).ok()?;
        String::from_utf8_lossy(&bytes).into_owned().into()
    }

    fn deobfuscate(raw_json: &str) -> Option<Value> {
        let array: Vec<String> = serde_json::from_str(raw_json).ok()?;
        let obfuscated = array.first()?;

        let step1 = Self::rot13(obfuscated);
        let step2 = Self::strip_markers(&step1);
        let step3 = Self::safe_b64_decode(&step2)?;
        let step4 = Self::shift_chars(&step3, 3);
        let step5: String = step4.chars().rev().collect();
        let step6 = Self::safe_b64_decode(&step5)?;

        serde_json::from_str(&step6).ok()
    }

    fn is_bait(url: &str) -> bool {
        let lower = url.to_lowercase();
        BAIT_PATTERNS.iter().any(|p| lower.contains(p))
    }

    fn extract_redirect(html: &str) -> Option<String> {
        let patterns = [
            r#"window\.location\.href\s*=\s*['"]([^'"]+)['"]"#,
            r#"window\.location\s*=\s*['"]([^'"]+)['"]"#,
            r#"location\.href\s*=\s*['"]([^'"]+)['"]"#,
        ];

        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(caps) = re.captures(html) {
                    return Some(caps[1].to_string());
                }
            }
        }
        None
    }

    fn extract_stream_url(html: &str) -> Option<String> {
        let json_re =
            Regex::new(r#"<script\s+type="application/json">\s*(\[.*?\])\s*</script>"#).ok()?;

        for caps in json_re.captures_iter(html) {
            if let Some(data) = Self::deobfuscate(&caps[1]) {
                if let Some(obj) = data.as_object() {
                    let url = obj
                        .get("direct_access_url")
                        .or_else(|| obj.get("source"))
                        .and_then(|v| v.as_str())
                        .map(String::from);

                    if let Some(ref u) = url {
                        if !Self::is_bait(u) {
                            return url;
                        }
                    }
                }
            }
        }

        let fallback_re = Regex::new(r#"(https?://[^\s"']+\.(?:mp4|m3u8)[^\s"']*)"#).ok()?;
        fallback_re
            .captures(html)
            .map(|c| c[1].to_string())
            .filter(|u| !Self::is_bait(u))
    }

    async fn fetch_page(&self, url: &str) -> Result<String, StreamError> {
        let response = self
            .client
            .get(url)
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(|e| StreamError::Network(e.to_string()))?;

        response
            .text()
            .await
            .map_err(|e| StreamError::Network(e.to_string()))
    }

    fn resolve_redirect_url(base_url: &str, redirect: &str) -> String {
        match redirect {
            r if r.starts_with("//") => format!("https:{}", r),
            r if r.starts_with("http") => r.to_string(),
            r => {
                if let Some(scheme_end) = base_url.find("://") {
                    let rest = &base_url[scheme_end + 3..];
                    let host_end = rest.find('/').unwrap_or(rest.len());
                    let scheme = &base_url[..scheme_end];
                    let host = &rest[..host_end];
                    format!("{}://{}{}", scheme, host, r)
                } else {
                    r.to_string()
                }
            }
        }
    }
}

impl Default for VoeResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StreamResolver for VoeResolver {
    fn name(&self) -> &str {
        "voe"
    }

    fn can_handle(&self, url: &str) -> bool {
        url.contains("voe.sx") || url.contains("voe.")
    }

    async fn resolve(&self, url: &str) -> Result<String, StreamError> {
        let mut current_url = url.to_string();

        for _ in 0..self.max_redirects {
            let html = self.fetch_page(&current_url).await?;

            if let Some(redirect) = Self::extract_redirect(&html) {
                current_url = Self::resolve_redirect_url(&current_url, &redirect);
                continue;
            }

            if let Some(stream_url) = Self::extract_stream_url(&html) {
                return Ok(stream_url);
            }

            break;
        }

        Err(StreamError::NotFound("Failed to extract stream URL".into()))
    }
}
