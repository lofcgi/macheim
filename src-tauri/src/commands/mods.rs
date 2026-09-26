use std::collections::HashSet;
use std::sync::Mutex;

use tauri::Emitter;
use tracing::info;

use crate::error::{AppError, AppResult};
use crate::models::InstalledMod;
use crate::services::{
    dependency_resolver, game_detector, mod_installer, profile_manager, thunderstore_client,
};
use crate::AppState;

/// Progress event payload sent to the frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProgressEvent {
    pub stage: String, // "resolving" | "downloading" | "installing" | "syncing"
    pub mod_name: String,
    pub current: usize, // current item index (1-based)
    pub total: usize,   // total items
    pub bytes_downloaded: u64,
    pub bytes_total: Option<u64>,
    pub message: String,
}

/// Install a mod and all its dependencies.
#[tauri::command]
pub async fn install_mod(
    full_name: String,
    version: Option<String>,
    state: tauri::State<'_, Mutex<AppState>>,
    app: tauri::AppHandle,
) -> AppResult<Vec<InstalledMod>> {
    info!("Command: install_mod({})", full_name);
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;
    profile_manager::validate_name(&full_name)?;

    let (game_path, packages, active_profile) = {
        let state = state
            .lock()
            .map_err(|e| AppError::Mod(format!("Failed to lock state: {}", e)))?;

        let game_path = state
            .game_path
            .clone()
            .ok_or_else(|| AppError::Mod("Game path not set".to_string()))?;

        let packages = state
            .thunderstore_cache
            .clone()
            .ok_or_else(|| AppError::Mod("Package cache not loaded".to_string()))?;

        let active_profile = state.active_profile.clone();

        (game_path, packages, active_profile)
    };

    let game_root = game_detector::get_valheim_root(&game_path);

    // Find the target package
    let target_pkg = thunderstore_client::find_package(&packages, &full_name)
        .ok_or_else(|| AppError::Mod(format!("Package '{}' not found", full_name)))?;

    let target_version = version
        .as_deref()
        .or_else(|| {
            target_pkg
                .versions
                .first()
                .map(|v| v.version_number.as_str())
        })
        .ok_or_else(|| AppError::Mod("No version available".to_string()))?;

    let target_ver_info = target_pkg
        .versions
        .iter()
        .find(|v| v.version_number == target_version)
        .ok_or_else(|| AppError::Mod("Version not found".to_string()))?;

    // Get currently installed mods to skip existing deps
    let profile = profile_manager::load_profile(&active_profile)?;
    let mut installed_set: HashSet<String> = profile.mods.iter().map(|m| m.full_name.clone()).collect();
    installed_set.insert("denikson-BepInExPack_Valheim".into());

    // Resolve dependencies
    emit_progress(
        &app,
        "resolving",
        &full_name,
        0,
        0,
        0,
        None,
        "Resolving dependencies...",
    );

    let deps = dependency_resolver::resolve_dependencies(
        &full_name,
        target_version,
        &packages,
        &profile.mods,
    )?;

    let total_items = deps.len() + 1; // deps + target mod
    let mut installed_mods = Vec::new();
    let mut failed_mods: Vec<String> = Vec::new();

    // Install dependencies first (in topological order)
    for (idx, dep) in deps.iter().enumerate() {
        if installed_set.contains(&dep.full_name) {
            continue;
        }

        emit_progress(
            &app,
            "downloading",
            &dep.full_name,
            idx + 1,
            total_items,
            0,
            None,
            &format!("Downloading {} ({}/{})", dep.name, idx + 1, total_items),
        );

        let app_clone = app.clone();
        let dep_name = dep.full_name.clone();
        let dep_idx = idx + 1;

        let dep_zip = match thunderstore_client::download_mod_with_progress(
            &dep.download_url,
            Some(Box::new(move |downloaded, total| {
                emit_progress(
                    &app_clone,
                    "downloading",
                    &dep_name,
                    dep_idx,
                    total_items,
                    downloaded,
                    total,
                    &format!("Downloading..."),
                );
            })),
        )
        .await
        {
            Ok(zip) => zip,
            Err(e) => {
                tracing::warn!("Failed to download {}: {}, skipping", dep.full_name, e);
                failed_mods.push(dep.full_name.clone());
                continue;
            }
        };

        emit_progress(
            &app,
            "installing",
            &dep.full_name,
            idx + 1,
            total_items,
            0,
            None,
            &format!("Installing {} ({}/{})", dep.name, idx + 1, total_items),
        );

        // Find dependency info for its own deps
        let dep_pkg = thunderstore_client::find_package(&packages, &dep.full_name);
        let dep_dependencies: Vec<String> = dep_pkg
            .and_then(|p| p.versions.first())
            .map(|v| v.dependencies.clone())
            .unwrap_or_default();

        match mod_installer::install_mod_from_bytes(
            &dep.author,
            &dep.name,
            &dep.version,
            &dep.description,
            &dep.icon,
            &dep_dependencies,
            &dep_zip,
            &game_root,
        ) {
            Ok(installed) => {
                profile_manager::add_mod_to_profile(&active_profile, installed.clone())?;
                installed_mods.push(installed);
            }
            Err(e) => {
                tracing::warn!("Failed to install {}: {}, skipping", dep.full_name, e);
                failed_mods.push(dep.full_name.clone());
            }
        }
    }

    // Install the target mod itself
    if !installed_set.contains(&full_name) {
        emit_progress(
            &app,
            "downloading",
            &full_name,
            total_items,
            total_items,
            0,
            None,
            &format!("Downloading {}", target_pkg.name),
        );

        let app_clone = app.clone();
        let fn_clone = full_name.clone();

        let target_zip = thunderstore_client::download_mod_with_progress(
            &target_ver_info.download_url,
            Some(Box::new(move |downloaded, total| {
                emit_progress(
                    &app_clone,
                    "downloading",
                    &fn_clone,
                    total_items,
                    total_items,
                    downloaded,
                    total,
                    "Downloading...",
                );
            })),
        )
        .await?;

        emit_progress(
            &app,
            "installing",
            &full_name,
            total_items,
            total_items,
            0,
            None,
            &format!("Installing {}", target_pkg.name),
        );

        let installed = mod_installer::install_mod_from_bytes(
            &target_pkg.owner,
            &target_pkg.name,
            &target_ver_info.version_number,
            &target_ver_info.description,
            &target_ver_info.icon,
            &target_ver_info.dependencies,
            &target_zip,
            &game_root,
        )?;

        profile_manager::add_mod_to_profile(&active_profile, installed.clone())?;
        installed_mods.push(installed);
    }

    emit_progress(
        &app,
        "done",
        &full_name,
        total_items,
        total_items,
        0,
        None,
        &format!(
            "Done! {} installed, {} failed",
            installed_mods.len(),
            failed_mods.len()
        ),
    );

    info!(
        "Successfully installed {} mods ({} failed)",
        installed_mods.len(),
        failed_mods.len()
    );
    crate::services::compatibility::reconcile(
        &profile_manager::load_profile(&active_profile)?,
        &game_root,
    )?;
    if !failed_mods.is_empty() {
        return Err(AppError::Mod(format!("Partially installed: {} succeeded. Failed dependencies: {}. Resolve these before playing; successful files were retained.", installed_mods.len(), failed_mods.join(", "))));
    }
    Ok(installed_mods)
}

/// Uninstall a mod.
#[tauri::command]
pub async fn uninstall_mod(
    full_name: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<()> {
    info!("Command: uninstall_mod({})", full_name);
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;
    profile_manager::validate_name(&full_name)?;

    let (game_path, active_profile) = {
        let state = state
            .lock()
            .map_err(|e| AppError::Mod(format!("Failed to lock state: {}", e)))?;
        let game_path = state
            .game_path
            .clone()
            .ok_or_else(|| AppError::Mod("Game path not set".to_string()))?;
        let active_profile = state.active_profile.clone();
        (game_path, active_profile)
    };

    let game_root = game_detector::get_valheim_root(&game_path);
    mod_installer::uninstall_mod(&full_name, &game_root)?;
    profile_manager::remove_mod_from_profile(&active_profile, &full_name)?;
    crate::services::compatibility::reconcile(
        &profile_manager::load_profile(&active_profile)?,
        &game_root,
    )?;

    Ok(())
}

/// Enable or disable a mod.
#[tauri::command]
pub async fn toggle_mod(
    full_name: String,
    enable: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<bool> {
    info!("Command: toggle_mod({}, enable={})", full_name, enable);
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;
    profile_manager::validate_name(&full_name)?;

    let (game_path, active_profile) = {
        let state = state
            .lock()
            .map_err(|e| AppError::Mod(format!("Failed to lock state: {}", e)))?;
        let game_path = state
            .game_path
            .clone()
            .ok_or_else(|| AppError::Mod("Game path not set".to_string()))?;
        let active_profile = state.active_profile.clone();
        (game_path, active_profile)
    };

    let game_root = game_detector::get_valheim_root(&game_path);
    let result = mod_installer::toggle_mod(&full_name, enable, &game_root)?;
    profile_manager::update_mod_enabled(&active_profile, &full_name, result)?;
    crate::services::compatibility::reconcile(
        &profile_manager::load_profile(&active_profile)?,
        &game_root,
    )?;

    Ok(result)
}

/// Get list of installed mods for the active profile.
#[tauri::command]
pub async fn get_installed_mods(
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Vec<InstalledMod>> {
    let active_profile = {
        let state = state
            .lock()
            .map_err(|e| AppError::Mod(format!("Failed to lock state: {}", e)))?;
        state.active_profile.clone()
    };

    let profile = profile_manager::load_profile(&active_profile)?;
    Ok(profile.mods)
}

/// Install a modpack.
#[tauri::command]
pub async fn install_modpack(
    full_name: String,
    state: tauri::State<'_, Mutex<AppState>>,
    app: tauri::AppHandle,
) -> AppResult<Vec<InstalledMod>> {
    info!("Command: install_modpack({})", full_name);
    install_mod(full_name, None, state, app).await
}

/// Sync mods: ensure all profile mods exist in the game directory,
/// and clean up unmanaged mods.
#[tauri::command]
pub async fn sync_mods(
    clean_unmanaged: Option<bool>,
    approved_unmanaged: Option<Vec<String>>,
    state: tauri::State<'_, Mutex<AppState>>,
    app: tauri::AppHandle,
) -> AppResult<SyncResult> {
    info!("Command: sync_mods");
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;

    let (game_path, packages, active_profile) = {
        let s = state
            .lock()
            .map_err(|e| AppError::Mod(format!("Lock: {}", e)))?;
        let gp = s
            .game_path
            .clone()
            .ok_or_else(|| AppError::Mod("Game not set".into()))?;
        let pkgs = s.thunderstore_cache.clone();
        (gp, pkgs, s.active_profile.clone())
    };

    let packages = match packages {
        Some(p) => p,
        None => {
            emit_progress(
                &app,
                "syncing",
                "",
                0,
                0,
                0,
                None,
                "Fetching package list...",
            );
            let source = profile_manager::load_profile(&active_profile)?.catalog_source;
            let p = thunderstore_client::fetch_catalog(source, false).await?;
            let mut s = state
                .lock()
                .map_err(|e| AppError::Mod(format!("Lock: {}", e)))?;
            s.thunderstore_cache = Some(p.clone());
            p
        }
    };

    let game_root = game_detector::get_valheim_root(&game_path);
    let plugins_dir = game_root.join("BepInEx/plugins");

    let profile = profile_manager::load_profile(&active_profile)?;

    // 1. Find missing mods
    let missing: Vec<_> = profile
        .mods
        .iter()
        .filter(|m| {
            if !m.enabled {
                return false;
            } // Sync must not re-enable disabled mods.
            let mod_dir = plugins_dir.join(&m.full_name);
            !mod_dir.exists() || dir_is_empty(&mod_dir)
        })
        .collect();

    let total = missing.len();
    let mut reinstalled = Vec::new();
    let mut failed = Vec::new();

    for (idx, m) in missing.iter().enumerate() {
        emit_progress(
            &app,
            "syncing",
            &m.full_name,
            idx + 1,
            total,
            0,
            None,
            &format!("Reinstalling {} ({}/{})", m.name, idx + 1, total),
        );

        let pkg = thunderstore_client::find_package(&packages, &m.full_name);
        if let Some(pkg) = pkg {
            let ver = pkg.versions.iter().find(|v| v.version_number == m.version);

            if let Some(ver) = ver {
                let app_clone = app.clone();
                let mod_name = m.full_name.clone();
                let idx_copy = idx + 1;

                match thunderstore_client::download_mod_with_progress(
                    &ver.download_url,
                    Some(Box::new(move |downloaded, total_bytes| {
                        emit_progress(
                            &app_clone,
                            "syncing",
                            &mod_name,
                            idx_copy,
                            total,
                            downloaded,
                            total_bytes,
                            "Downloading...",
                        );
                    })),
                )
                .await
                {
                    Ok(zip) => {
                        match mod_installer::install_mod_from_bytes(
                            &pkg.owner,
                            &pkg.name,
                            &ver.version_number,
                            &ver.description,
                            &ver.icon,
                            &ver.dependencies,
                            &zip,
                            &game_root,
                        ) {
                            Ok(_) => reinstalled.push(m.full_name.clone()),
                            Err(e) => {
                                tracing::warn!("Reinstall failed {}: {}", m.full_name, e);
                                failed.push(m.full_name.clone());
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Download failed {}: {}", m.full_name, e);
                        failed.push(m.full_name.clone());
                    }
                }
            } else {
                failed.push(m.full_name.clone());
            }
        } else {
            failed.push(m.full_name.clone());
        }
    }

    // 2. Clean unmanaged mods
    let mut cleaned = Vec::new();
    if clean_unmanaged.unwrap_or(false) {
        let profile_names: HashSet<String> =
            profile.mods.iter().map(|m| m.full_name.clone()).collect();
        let approved: HashSet<String> =
            approved_unmanaged.unwrap_or_default().into_iter().collect();
        let recovery = game_root.join("BepInEx/.macheim-clean-backups").join(
            chrono::Utc::now()
                .timestamp_nanos_opt()
                .unwrap_or_default()
                .to_string(),
        );

        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if entry.file_type()?.is_dir()
                    && !name.starts_with('.')
                    && name != crate::services::compatibility::MANAGED_DIR
                    && !profile_names.contains(&name)
                    && approved.contains(&name)
                {
                    crate::services::compatibility::reject_symlink_ancestors(&entry.path())?;
                    std::fs::create_dir_all(&recovery)?;
                    std::fs::rename(entry.path(), recovery.join(&name))?;
                    cleaned.push(name);
                }
            }
        }
    }

    emit_progress(
        &app,
        "done",
        "",
        0,
        0,
        0,
        None,
        &format!(
            "Sync complete: {} reinstalled, {} failed, {} cleaned",
            reinstalled.len(),
            failed.len(),
            cleaned.len()
        ),
    );

    let result = SyncResult {
        reinstalled,
        failed,
        cleaned,
    };
    crate::services::compatibility::reconcile(
        &profile_manager::load_profile(&active_profile)?,
        &game_root,
    )?;
    Ok(result)
}

fn dir_is_empty(path: &std::path::Path) -> bool {
    if path.is_file() {
        return false;
    }
    match std::fs::read_dir(path) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => true,
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SyncResult {
    pub reinstalled: Vec<String>,
    pub failed: Vec<String>,
    pub cleaned: Vec<String>,
}

/// List mods in the plugins directory that are not tracked by the current profile.
/// Used by the frontend to show a confirmation dialog before cleaning.
#[tauri::command]
pub async fn list_unmanaged_mods(
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Vec<String>> {
    let (game_path, active_profile) = {
        let s = state
            .lock()
            .map_err(|e| AppError::Mod(format!("Lock: {}", e)))?;
        let gp = s
            .game_path
            .clone()
            .ok_or_else(|| AppError::Mod("Game not set".into()))?;
        (gp, s.active_profile.clone())
    };

    let game_root = game_detector::get_valheim_root(&game_path);
    let plugins_dir = game_root.join("BepInEx/plugins");

    let profile = profile_manager::load_profile(&active_profile)?;

    let profile_names: HashSet<String> = profile.mods.iter().map(|m| m.full_name.clone()).collect();
    let mut unmanaged = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type()?.is_dir()
                && !name.starts_with('.')
                && name != crate::services::compatibility::MANAGED_DIR
                && !profile_names.contains(&name)
            {
                unmanaged.push(name);
            }
        }
    }

    Ok(unmanaged)
}

fn emit_progress(
    app: &tauri::AppHandle,
    stage: &str,
    mod_name: &str,
    current: usize,
    total: usize,
    bytes_downloaded: u64,
    bytes_total: Option<u64>,
    message: &str,
) {
    let _ = app.emit(
        "mod-progress",
        ProgressEvent {
            stage: stage.to_string(),
            mod_name: mod_name.to_string(),
            current,
            total,
            bytes_downloaded,
            bytes_total,
            message: message.to_string(),
        },
    );
}
