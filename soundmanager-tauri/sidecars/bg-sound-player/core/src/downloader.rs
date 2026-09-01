//! GitHub sound schemes downloader. Searches the Sound-Manager-Schemes repo
//! (ORelio) and downloads selected .ths/.zip files into the local schemes folder.

use crate::errors::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubScheme {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

#[derive(Deserialize)]
struct GhContent {
    name: String,
    download_url: Option<String>,
    size: u64,
    #[serde(rename = "type")]
    kind: String,
}

/// Search GitHub: list files in Sound-Manager-Schemes, filter by query.
pub async fn search_schemes(query: Option<&str>) -> CoreResult<Vec<GithubScheme>> {
    let client = reqwest::Client::builder()
        .user_agent("SoundManager/4.0")
        .timeout(std::time::Duration::from_secs(12))
        .build()
        .map_err(|e| CoreError::Other(e.to_string()))?;

    let url = "https://api.github.com/repos/ORelio/Sound-Manager-Schemes/contents";
    let items: Vec<GhContent> = client
        .get(url)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?
        .json()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?;

    let q = query.unwrap_or("").to_lowercase();
    Ok(items
        .into_iter()
        .filter(|c| c.kind == "file" && c.download_url.is_some())
        .filter(|c| q.is_empty() || c.name.to_lowercase().contains(&q))
        .map(|c| GithubScheme {
            name: c.name,
            download_url: c.download_url.unwrap(),
            size: c.size,
        })
        .collect())
}

pub async fn download_scheme(download_url: &str, file_name: &str) -> CoreResult<std::path::PathBuf> {
    let client = reqwest::Client::builder()
        .user_agent("SoundManager/4.0")
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| CoreError::Other(e.to_string()))?;
    let bytes = client
        .get(download_url)
        .send()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?
        .bytes()
        .await
        .map_err(|e| CoreError::Other(e.to_string()))?;

    let schemes_dir = {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        std::path::PathBuf::from(appdata).join("SoundManager").join("Schemes")
    };
    std::fs::create_dir_all(&schemes_dir)?;
    let dest = schemes_dir.join(file_name);
    std::fs::write(&dest, &bytes)?;
    Ok(dest)
}
