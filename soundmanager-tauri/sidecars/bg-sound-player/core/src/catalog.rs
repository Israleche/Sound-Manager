//! Sound catalog: MyInstants search + direct MP3 URLs.
//! Chosen as best source among the four candidates:
//! - MyInstants: simple HTML, predictable /media/sounds/*.mp3 URLs, no login,
//!   large library, search via /en/search/?name=query. Other sites (Lab/World/BoardButtons)
//!   are JS-heavy or lack direct download links. MyInstants wins on parsability.

use crate::errors::{CoreError, CoreResult};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CatalogSound {
    pub title: String,
    pub url: String,
    pub page_url: String,
}

/// Search MyInstants for sounds matching `query`. Parses play('/media/sounds/...') calls.
pub async fn search_myinstants(query: &str) -> CoreResult<Vec<CatalogSound>> {
    let client = reqwest::Client::builder()
        .user_agent("SoundManager/4.0")
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| CoreError::Other(e.to_string()))?;

    let url = format!(
        "https://www.myinstants.com/en/search/?name={}",
        urlencoding::encode(query)
    );
    let html = client
        .get(&url)
        .send()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?
        .text()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?;

    Ok(parse_myinstants(&html))
}

pub fn parse_myinstants(html: &str) -> Vec<CatalogSound> {
    let mut out = Vec::new();
    // Pattern: play('/media/sounds/xxx.mp3', ...) + <a href="/en/instant/slug/">
    for (i, _) in html.match_indices("play('/media/sounds/") {
        let start = i + "play('".len();
        let Some(end) = html[start..].find('\'') else { continue };
        let mp3_path = &html[start..start + end];
        let url = format!("https://www.myinstants.com{mp3_path}");

        // Title: find nearest instant-link after this play() call
        let slice = &html[i..];
        let title = slice
            .find("instant-link")
            .and_then(|p| {
                let a = &slice[p..];
                let gt = a.find('>')? + 1;
                let lt = a[gt..].find('<')?;
                Some(a[gt..gt + lt].trim().to_string())
            })
            .unwrap_or_else(|| {
                mp3_path
                    .rsplit('/')
                    .next()
                    .unwrap_or("sound")
                    .trim_end_matches(".mp3")
                    .to_string()
            });

        // Page URL: href="/en/instant/slug/"
        let page_url = slice
            .find("href=\"/en/instant/")
            .and_then(|p| {
                let a = &slice[p + 6..];
                let q = a.find('"')?;
                Some(format!("https://www.myinstants.com{}", &a[..q]))
            })
            .unwrap_or_default();

        out.push(CatalogSound { title, url, page_url });
        if out.len() >= 30 {
            break;
        }
    }
    out
}

/// Download an MP3 from a catalog URL into a temp file, return the temp path.
pub async fn download_to_temp(url: &str) -> CoreResult<std::path::PathBuf> {
    let client = reqwest::Client::builder()
        .user_agent("SoundManager/4.0")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| CoreError::Other(e.to_string()))?;
    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?;
    let name = url
        .rsplit('/')
        .next()
        .unwrap_or("catalog.mp3")
        .to_string();
    let path = std::env::temp_dir().join(format!("sm-catalog-{}-{}", std::process::id(), name));
    std::fs::write(&path, &bytes)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_sample() {
        let html = r#"<button onclick="play('/media/sounds/hello.mp3', 'x', 'y')"></button><a href="/en/instant/hello/" class="instant-link">Hello</a>"#;
        let v = parse_myinstants(html);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].url, "https://www.myinstants.com/media/sounds/hello.mp3");
        assert_eq!(v[0].title, "Hello");
    }
}
