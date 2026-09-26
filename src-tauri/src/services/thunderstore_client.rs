use std::io::Read;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use tracing::{debug, info};

use crate::error::{AppError, AppResult};
use crate::models::thunderstore::{PackageListing, ThunderstorePackage};

const THUNDERSTORE_INDEX_URL: &str =
    "https://thunderstore.io/c/valheim/api/v1/package-listing-index/";
const CACHE_MAX_AGE_MINUTES: i64 = 30;

/// Get the application data directory for cache and config storage.
pub fn get_app_data_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    home.join("Library/Application Support/com.macheim")
}

/// Get the cache directory for Thunderstore data.
fn get_cache_dir() -> PathBuf {
    get_app_data_dir().join("cache/thunderstore")
}

/// Get the path to the cached packages file.
fn get_cache_file() -> PathBuf {
    get_cache_dir().join("packages.json")
}

/// Check if the cache is still valid (less than 30 minutes old).
fn is_cache_valid() -> bool {
    let cache_file = get_cache_file();
    if !cache_file.exists() {
        return false;
    }

    match std::fs::metadata(&cache_file) {
        Ok(meta) => {
            if let Ok(modified) = meta.modified() {
                let modified_dt: DateTime<Utc> = modified.into();
                let age = Utc::now() - modified_dt;
                age.num_minutes() < CACHE_MAX_AGE_MINUTES
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

/// Load packages from the disk cache.
fn load_cache() -> AppResult<Vec<ThunderstorePackage>> {
    let cache_file = get_cache_file();
    let content = std::fs::read_to_string(&cache_file)?;
    let packages: Vec<ThunderstorePackage> = serde_json::from_str(&content)?;
    debug!("Loaded {} packages from cache", packages.len());
    Ok(packages)
}

/// Save packages to the disk cache.
fn save_cache(packages: &[ThunderstorePackage]) -> AppResult<()> {
    let cache_dir = get_cache_dir();
    std::fs::create_dir_all(&cache_dir)?;
    let cache_file = get_cache_file();
    let content = serde_json::to_string(packages)?;
    super::compatibility::atomic_write(&cache_file, content.as_bytes())?;
    debug!("Saved {} packages to cache", packages.len());
    Ok(())
}

/// Fetch all Valheim packages from Thunderstore API.
/// Uses disk cache if available and fresh (< 30 minutes).
pub async fn fetch_packages(force_refresh: bool) -> AppResult<Vec<ThunderstorePackage>> {
    // Check cache first
    if !force_refresh && is_cache_valid() {
        info!("Using cached Thunderstore data");
        match load_cache() {
            Ok(packages) => return Ok(packages),
            Err(e) => {
                debug!("Cache load failed, fetching fresh: {}", e);
            }
        }
    }

    info!("Fetching packages from Thunderstore API...");
    let packages = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        fetch_index(THUNDERSTORE_INDEX_URL),
    ).await.map_err(|_| AppError::Network("Catalog refresh timed out. Check your connection and retry; installed mods were not changed.".into()))??;

    info!("Fetched {} packages from Thunderstore", packages.len());

    // Save to cache
    if let Err(e) = save_cache(&packages) {
        debug!("Failed to save cache: {}", e);
    }

    Ok(packages)
}

pub async fn fetch_catalog(
    source: crate::models::profile::CatalogSource,
    force: bool,
) -> AppResult<Vec<ThunderstorePackage>> {
    if source == crate::models::profile::CatalogSource::Thunderstore {
        return fetch_packages(force).await;
    }
    let cache = get_app_data_dir().join("cache/hexium/packages.json");
    if !force
        && cache
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|time| {
                time.elapsed()
                    .is_ok_and(|age| age < std::time::Duration::from_secs(1800))
            })
    {
        return decode_json(&std::fs::read(cache)?);
    }
    let packages = tokio::time::timeout(
        std::time::Duration::from_secs(180),
        fetch_index("https://valheim.hexium.gg/api/v1/package-listing-index/"),
    )
    .await
    .map_err(|_| AppError::Network("Hexium catalog timed out. Please retry.".into()))??;
    if let Err(e) = super::compatibility::atomic_write(&cache, &serde_json::to_vec(&packages)?) {
        tracing::warn!("Could not save Hexium catalog cache: {e}");
    }
    Ok(packages)
}

fn http_client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("Macheim/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(15))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::Network(e.to_string()))
}

// Listing indexes/chunks are gzip files, even without Content-Encoding: gzip.
fn decode_json<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> AppResult<T> {
    let mut decoded = Vec::new();
    if bytes.starts_with(&[0x1f, 0x8b]) {
        flate2::read::GzDecoder::new(bytes)
            .take(128 * 1024 * 1024 + 1)
            .read_to_end(&mut decoded)
            .map_err(|e| {
                AppError::Network(format!("Incomplete compressed catalog: {e}. Please retry."))
            })?;
    } else {
        decoded.extend_from_slice(bytes);
    }
    if decoded.len() > 128 * 1024 * 1024 {
        return Err(AppError::Network(
            "Catalog chunk exceeds safety limit".into(),
        ));
    }
    serde_json::from_slice(&decoded).map_err(|e| {
        AppError::Network(format!(
            "Invalid catalog data: {e}. Please retry; installed mods were not changed."
        ))
    })
}

async fn get_json<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> AppResult<T> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::Network(format!("Could not contact package server: {e}")))?
        .error_for_status()
        .map_err(|e| AppError::Network(format!("Package server rejected request: {e}")))?;
    let bytes = response.bytes().await.map_err(|e| {
        AppError::Network(format!("Catalog download interrupted: {e}. Please retry."))
    })?;
    decode_json(&bytes)
}

async fn fetch_index(index: &str) -> AppResult<Vec<ThunderstorePackage>> {
    use futures_util::{stream, StreamExt, TryStreamExt};
    let client = http_client()?;
    let urls: Vec<String> = get_json(&client, index).await?;
    if urls.is_empty() || urls.len() > 256 {
        return Err(AppError::Network("Invalid catalog index size".into()));
    }
    for url in &urls {
        let parsed = reqwest::Url::parse(url).map_err(|e| AppError::Network(e.to_string()))?;
        let host = parsed.host_str().unwrap_or_default();
        if parsed.scheme() != "https"
            || !(host == "thunderstore.io"
                || host.ends_with(".thunderstore.io")
                || host == "valheim.hexium.gg")
        {
            return Err(AppError::Network("Untrusted catalog chunk URL".into()));
        }
    }
    let chunks: Vec<Vec<ThunderstorePackage>> = stream::iter(urls)
        .map(|url| {
            let client = &client;
            async move { get_json(client, &url).await }
        })
        .buffered(4)
        .try_collect()
        .await?;
    let packages: Vec<_> = chunks.into_iter().flatten().collect();
    if packages.is_empty() {
        return Err(AppError::Network(
            "Package server returned an empty catalog. Please retry.".into(),
        ));
    }
    Ok(packages)
}

/// Bootstrap the loader without downloading the full mod catalog.
pub async fn fetch_bepinex_package() -> AppResult<ThunderstorePackage> {
    #[derive(serde::Deserialize)]
    struct Response {
        name: String,
        full_name: String,
        owner: String,
        package_url: String,
        date_updated: String,
        is_deprecated: bool,
        rating_score: i64,
        latest: crate::models::thunderstore::PackageVersion,
    }
    let p: Response = get_json(
        &http_client()?,
        "https://thunderstore.io/api/experimental/package/denikson/BepInExPack_Valheim/",
    )
    .await?;
    Ok(ThunderstorePackage {
        name: p.name,
        full_name: p.full_name,
        owner: p.owner,
        package_url: p.package_url,
        date_updated: p.date_updated,
        is_deprecated: p.is_deprecated,
        rating_score: p.rating_score,
        versions: vec![p.latest],
        categories: vec![],
        is_pinned: false,
    })
}

/// Search cached packages by query string.
/// Matches against name, description, and owner (case-insensitive contains).
pub fn search_packages(packages: &[ThunderstorePackage], query: &str) -> Vec<PackageListing> {
    let query_lower = query.to_lowercase();
    let terms: Vec<&str> = query_lower.split_whitespace().collect();

    packages
        .iter()
        .filter(|pkg| {
            if terms.is_empty() {
                return true;
            }
            let name_lower = pkg.name.to_lowercase();
            let owner_lower = pkg.owner.to_lowercase();
            let full_name_lower = pkg.full_name.to_lowercase();
            let desc_lower = pkg
                .versions
                .first()
                .map(|v| v.description.to_lowercase())
                .unwrap_or_default();

            terms.iter().all(|term| {
                name_lower.contains(term)
                    || owner_lower.contains(term)
                    || full_name_lower.contains(term)
                    || desc_lower.contains(term)
            })
        })
        .map(PackageListing::from)
        .collect()
}

/// Find a specific package by full name (e.g., "denikson-BepInExPack_Valheim").
pub fn find_package<'a>(
    packages: &'a [ThunderstorePackage],
    full_name: &str,
) -> Option<&'a ThunderstorePackage> {
    packages.iter().find(|p| p.full_name == full_name)
}

/// Download a mod's ZIP file and return the bytes. No timeout on download body.
pub async fn download_mod(download_url: &str) -> AppResult<Vec<u8>> {
    download_mod_with_progress(download_url, None).await
}

/// Progress callback type: (downloaded_bytes, total_bytes_option)
pub type ProgressFn = Box<dyn Fn(u64, Option<u64>) + Send>;

/// Download a mod's ZIP with optional progress callback. No body timeout.
pub async fn download_mod_with_progress(
    download_url: &str,
    progress: Option<ProgressFn>,
) -> AppResult<Vec<u8>> {
    info!("Downloading mod from: {}", download_url);
    let cdn = super::download_settings::load()?;
    download_from_cdn(download_url, progress, cdn).await
}

async fn download_from_cdn(download_url: &str, progress: Option<ProgressFn>, cdn: super::download_settings::DownloadCdn) -> AppResult<Vec<u8>> {

    let client = reqwest::Client::builder()
        .user_agent(concat!("Macheim/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(std::time::Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        // No overall timeout - large mods can be 200MB+
        .build()
        .map_err(|e| AppError::Network(format!("Failed to create HTTP client: {}", e)))?;

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(45),
        async {
            let mut url = reqwest::Url::parse(download_url).map_err(|e| AppError::Network(e.to_string()))?;
            for _ in 0..10 {
                url = super::download_settings::rewrite(&url, cdn)?;
                if url.scheme() != "https" { return Err(AppError::Network("Refusing a non-HTTPS package download".into())); }
                let response = client.get(url.clone()).send().await.map_err(|e| AppError::Network(format!("Download from {} failed: {e}. Try an explicit Thunderstore CDN in Settings; do not disable security software.", url.host_str().unwrap_or("unknown host"))))?;
                if !response.status().is_redirection() { return Ok(response); }
                let location = response.headers().get(reqwest::header::LOCATION)
                    .ok_or_else(|| AppError::Network("Download redirect has no Location".into()))?
                    .to_str().map_err(|e| AppError::Network(e.to_string()))?;
                url = url.join(location).map_err(|e| AppError::Network(e.to_string()))?;
            }
            Err(AppError::Network("Too many download redirects".into()))
        },
    )
    .await
    .map_err(|_| {
        AppError::Network("Download server did not respond within 45 seconds. Please retry.".into())
    })?
    ?;

    if !response.status().is_success() {
        return Err(AppError::Network(format!(
            "Download returned status: {}",
            response.status()
        )));
    }

    let total_size = response.content_length();

    // Stream the response body
    let mut bytes =
        Vec::with_capacity(total_size.unwrap_or(1024 * 1024).min(8 * 1024 * 1024) as usize);
    let mut stream = response.bytes_stream();

    use futures_util::StreamExt;
    while let Some(chunk) = tokio::time::timeout(std::time::Duration::from_secs(45), stream.next()).await
        .map_err(|_| AppError::Network("Download stalled for 45 seconds. Please retry; do not disable your security software.".into()))? {
        let chunk = chunk.map_err(|e| AppError::Network(format!("Download stream error: {}", e)))?;
        bytes.extend_from_slice(&chunk);
        if let Some(ref cb) = progress {
            cb(bytes.len() as u64, total_size);
        }
    }

    info!("Downloaded {} bytes", bytes.len());
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[tokio::test]
    #[ignore = "live CDN download comparison; no package is installed or executed"]
    async fn live_explicit_cdn_downloads_match() {
        use super::super::download_settings::DownloadCdn;
        use sha2::{Digest, Sha256};
        let url="https://thunderstore.io/package/download/denikson/BepInExPack_Valheim/5.4.2351/";
        let primary=download_from_cdn(url,None,DownloadCdn::Automatic).await.unwrap();
        let alternate=download_from_cdn(url,None,DownloadCdn::Hetzner).await.unwrap();
        assert_eq!(Sha256::digest(&primary),Sha256::digest(&alternate));
        assert!(zip::ZipArchive::new(std::io::Cursor::new(alternate)).is_ok());
        println!("Default and explicit Hetzner downloads match ({} bytes)",primary.len());
    }
    #[test]
    fn decodes_plain_and_gzip_and_rejects_html_or_truncated_data() {
        let json = br#"["ok"]"#;
        assert_eq!(decode_json::<Vec<String>>(json).unwrap(), vec!["ok"]);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        gz.write_all(json).unwrap();
        let bytes = gz.finish().unwrap();
        assert_eq!(decode_json::<Vec<String>>(&bytes).unwrap(), vec!["ok"]);
        assert!(decode_json::<Vec<String>>(&bytes[..8]).is_err());
        assert!(decode_json::<Vec<String>>(b"<html>blocked</html>").is_err());
    }
    #[test]
    fn catalog_accepts_negative_rating_scores() {
        let data = br#"{"name":"Example","full_name":"Team-Example","owner":"Team","package_url":"https://thunderstore.io/","date_updated":"2026-09-24","is_deprecated":false,"rating_score":-1,"versions":[]}"#;
        assert!(decode_json::<ThunderstorePackage>(data).is_ok());
    }
    #[tokio::test]
    #[ignore = "live service check; run explicitly before release"]
    async fn live_catalog_and_loader_decode() {
        let packages = fetch_index(THUNDERSTORE_INDEX_URL).await.unwrap();
        assert!(packages.len() > 100);
        assert!(find_package(&packages, "denikson-BepInExPack_Valheim").is_some());
        let loader = fetch_bepinex_package().await.unwrap();
        assert_eq!(loader.full_name, "denikson-BepInExPack_Valheim");
        println!(
            "Decoded {} packages; loader {}",
            packages.len(),
            loader.versions[0].version_number
        );
        let hexium = fetch_index("https://valheim.hexium.gg/api/v1/package-listing-index/")
            .await
            .unwrap();
        assert!(hexium.len() > 100);
        assert!(find_package(&hexium, "ValheimModding-Jotunn").is_some());
        println!("Decoded {} Hexium packages", hexium.len());
        for package in [
            &loader,
            find_package(&hexium, "ValheimModding-Jotunn").unwrap(),
        ] {
            let bytes = download_mod(&package.versions[0].download_url)
                .await
                .unwrap();
            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&bytes)).unwrap();
            assert!(zip.by_name("manifest.json").is_ok());
            println!(
                "Downloaded and ZIP-validated {}: {} bytes (not installed)",
                package.full_name,
                bytes.len()
            );
        }
    }
}
