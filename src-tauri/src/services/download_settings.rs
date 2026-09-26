use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DownloadCdn {
    #[default]
    Automatic,
    Cloudflare,
    Google,
    Hetzner,
}

pub fn load() -> AppResult<DownloadCdn> {
    let path = super::thunderstore_client::get_app_data_dir().join("download-cdn.json");
    match std::fs::read(path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(DownloadCdn::Automatic),
        Err(e) => Err(e.into()),
    }
}

pub fn rewrite(url: &reqwest::Url, cdn: DownloadCdn) -> AppResult<reqwest::Url> {
    let mut url = url.clone();
    let host = match cdn {
        DownloadCdn::Automatic => return Ok(url),
        DownloadCdn::Cloudflare => "ccdn.thunderstore.io",
        DownloadCdn::Google => "gcdn.thunderstore.io",
        DownloadCdn::Hetzner => "hcdn-1.hcdn.thunderstore.io",
    };
    if url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some("ccdn.thunderstore.io" | "gcdn.thunderstore.io" | "hcdn-1.hcdn.thunderstore.io")
        )
        && url.path().starts_with("/live/repository/packages/")
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
    {
        url.set_host(Some(host))
            .map_err(|e| AppError::Network(e.to_string()))?;
    }
    Ok(url)
}

#[tauri::command]
pub fn get_download_cdn() -> AppResult<DownloadCdn> {
    load()
}

#[tauri::command]
pub fn set_download_cdn(cdn: DownloadCdn) -> AppResult<()> {
    super::compatibility::atomic_write(
        &super::thunderstore_client::get_app_data_dir().join("download-cdn.json"),
        &serde_json::to_vec(&cdn)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_explicit_trusted_package_cdn_is_rewritten() {
        let url = reqwest::Url::parse(
            "https://gcdn.thunderstore.io/live/repository/packages/Team-Mod-1.0.0.zip",
        )
        .unwrap();
        assert_eq!(rewrite(&url, DownloadCdn::Automatic).unwrap(), url);
        assert_eq!(
            rewrite(&url, DownloadCdn::Hetzner).unwrap().host_str(),
            Some("hcdn-1.hcdn.thunderstore.io")
        );
        for raw in [
            "https://valheim.hexium.gg/package/test",
            "https://gcdn.thunderstore.io.evil.test/live/repository/packages/test",
            "http://gcdn.thunderstore.io/live/repository/packages/test",
            "https://gcdn.thunderstore.io/api/test",
        ] {
            let url = reqwest::Url::parse(raw).unwrap();
            assert_eq!(rewrite(&url, DownloadCdn::Hetzner).unwrap(), url);
        }
        assert!(serde_json::from_str::<DownloadCdn>("\"untrusted\"").is_err());
    }
}
