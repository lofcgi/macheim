use std::sync::Mutex;

use tracing::info;

use crate::error::{AppError, AppResult};
use crate::models::thunderstore::{PackageListing, ThunderstorePackage};
use crate::services::thunderstore_client;
use crate::AppState;

/// Fetch all packages from Thunderstore (uses cache if fresh).
#[tauri::command]
pub async fn fetch_packages(
    force_refresh: Option<bool>,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Vec<PackageListing>> {
    info!(
        "Command: fetch_packages (force={})",
        force_refresh.unwrap_or(false)
    );

    let profile_name = state
        .lock()
        .map_err(|e| AppError::Profile(e.to_string()))?
        .active_profile
        .clone();
    let packages = thunderstore_client::fetch_all_catalogs(force_refresh.unwrap_or(false)).await?;

    // Create listings for the frontend (lightweight)
    let listings: Vec<PackageListing> = packages.iter().map(PackageListing::from).collect();

    // Cache in state
    let mut state = state
        .lock()
        .map_err(|e| AppError::Network(format!("Failed to lock state: {}", e)))?;
    if state.active_profile != profile_name {
        return Err(AppError::Profile(
            "Profile changed during catalog refresh. Refresh again for the active profile.".into(),
        ));
    }
    state.thunderstore_cache = Some(packages);
    state.cache_updated_at = Some(chrono::Utc::now());

    info!("Cached {} packages in state", listings.len());
    Ok(listings)
}

/// Search packages by query string.
#[tauri::command]
pub async fn search_packages(
    query: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Vec<PackageListing>> {
    let state = state
        .lock()
        .map_err(|e| AppError::Network(format!("Failed to lock state: {}", e)))?;

    let packages = state.thunderstore_cache.as_ref().ok_or_else(|| {
        AppError::Network("Package cache not loaded. Fetch packages first.".to_string())
    })?;

    let results = thunderstore_client::search_packages(packages, &query);
    Ok(results)
}

/// Get full details for a specific package by full_name.
#[tauri::command]
pub async fn get_package_details(
    full_name: String,
    source: Option<crate::models::profile::CatalogSource>,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<ThunderstorePackage> {
    let state = state
        .lock()
        .map_err(|e| AppError::Network(format!("Failed to lock state: {}", e)))?;

    let packages = state
        .thunderstore_cache
        .as_ref()
        .ok_or_else(|| AppError::Network("Package cache not loaded".to_string()))?;

    let mut matches = packages.iter().filter(|p| p.full_name == full_name && source.is_none_or(|s| s == p.source));
    let pkg = matches.next().ok_or_else(|| AppError::Network(format!("Package '{}' not found in selected source", full_name)))?;
    if matches.next().is_some() { return Err(AppError::Mod("Select Thunderstore or Hexium for this package".into())); }
    Ok(pkg.clone())
}
