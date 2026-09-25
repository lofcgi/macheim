use std::sync::Mutex;

use tracing::info;

use crate::error::{AppError, AppResult};
use crate::services::bepinex_installer::{self, BepInExStatus};
use crate::services::game_detector;
use crate::services::thunderstore_client;
use crate::AppState;

/// Install BepInEx to the Valheim directory.
#[tauri::command]
pub async fn install_bepinex(state: tauri::State<'_, Mutex<AppState>>) -> AppResult<BepInExStatus> {
    info!("Command: install_bepinex");
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;

    let game_path = {
        let s = state
            .lock()
            .map_err(|e| AppError::BepInEx(format!("Failed to lock state: {}", e)))?;
        s.game_path
            .clone()
            .ok_or_else(|| AppError::BepInEx("Game path not set. Detect game first.".to_string()))?
    };

    // Auto-fetch packages if cache is not loaded
    let packages = {
        let s = state
            .lock()
            .map_err(|e| AppError::BepInEx(format!("Failed to lock state: {}", e)))?;
        s.thunderstore_cache.clone()
    };

    let packages = match packages {
        Some(pkgs) => pkgs,
        None => {
            vec![thunderstore_client::fetch_bepinex_package().await?]
        }
    };

    let game_root = game_detector::get_valheim_root(&game_path);

    bepinex_installer::install_bepinex(&game_root, &packages).await?;

    let status = bepinex_installer::check_bepinex_status(&game_root);

    // Update state
    let mut s = state
        .lock()
        .map_err(|e| AppError::BepInEx(format!("Failed to lock state: {}", e)))?;
    s.bepinex_installed = status.installed;

    Ok(status)
}

/// Get current BepInEx installation status.
#[tauri::command]
pub async fn get_bepinex_status(
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<BepInExStatus> {
    let state = state
        .lock()
        .map_err(|e| AppError::BepInEx(format!("Failed to lock state: {}", e)))?;

    let game_path = state
        .game_path
        .as_ref()
        .ok_or_else(|| AppError::BepInEx("Game path not set".to_string()))?;

    let game_root = game_detector::get_valheim_root(game_path);
    Ok(bepinex_installer::check_bepinex_status(&game_root))
}

/// Uninstall BepInEx from the Valheim directory.
#[tauri::command]
pub async fn uninstall_bepinex(
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<BepInExStatus> {
    info!("Command: uninstall_bepinex");
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;

    let game_path = {
        let state = state
            .lock()
            .map_err(|e| AppError::BepInEx(format!("Failed to lock state: {}", e)))?;
        state
            .game_path
            .clone()
            .ok_or_else(|| AppError::BepInEx("Game path not set".to_string()))?
    };

    let game_root = game_detector::get_valheim_root(&game_path);
    bepinex_installer::uninstall_bepinex(&game_root)?;

    let status = bepinex_installer::check_bepinex_status(&game_root);

    let mut state = state
        .lock()
        .map_err(|e| AppError::BepInEx(format!("Failed to lock state: {}", e)))?;
    state.bepinex_installed = status.installed;

    Ok(status)
}
