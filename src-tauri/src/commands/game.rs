use std::sync::Mutex;

use tracing::info;

use crate::error::AppResult;
use crate::services::game_detector::{self, GameStatus};
use crate::services::profile_manager;
use crate::AppState;

/// Detect the Valheim installation and store the path in app state.
#[tauri::command]
pub async fn detect_game(state: tauri::State<'_, Mutex<AppState>>) -> AppResult<GameStatus> {
    info!("Command: detect_game");
    let _operation = crate::lock_operation(&state)?;

    match game_detector::detect_valheim() {
        Ok(path) => {
            let game_root = game_detector::get_valheim_root(&path);

            // Check BepInEx status
            let bepinex_installed =
                crate::services::bepinex_installer::check_bepinex_status(&game_root).installed;

            let first_detection = state
                .lock()
                .map_err(|e| crate::error::AppError::Profile(e.to_string()))?
                .game_path
                .is_none();
            let recovered_profile = if first_detection && bepinex_installed {
                Some(profile_manager::initialize_game_profile(&game_root)?)
            } else {
                None
            };
            {
                let mut state = state.lock().map_err(|e| {
                    crate::error::AppError::GameNotFound(format!("Failed to lock state: {}", e))
                })?;
                state.game_path = Some(path.clone());
                state.bepinex_installed = bepinex_installed;
                if let Some(name) = recovered_profile {
                    state.active_profile = name;
                }
            }

            let state = state.lock().map_err(|e| {
                crate::error::AppError::GameNotFound(format!("Failed to lock state: {}", e))
            })?;
            let status = game_detector::get_game_status_info(
                &Some(path),
                bepinex_installed,
                &state.active_profile,
            );
            Ok(status)
        }
        Err(e) => {
            let mut state = state.lock().map_err(|e| {
                crate::error::AppError::GameNotFound(format!("Failed to lock state: {}", e))
            })?;
            state.game_path = None;
            state.bepinex_installed = false;

            Err(e)
        }
    }
}

/// Get the current game status without re-detecting.
#[tauri::command]
pub async fn get_game_status(state: tauri::State<'_, Mutex<AppState>>) -> AppResult<GameStatus> {
    let state = state.lock().map_err(|e| {
        crate::error::AppError::GameNotFound(format!("Failed to lock state: {}", e))
    })?;

    Ok(game_detector::get_game_status_info(
        &state.game_path,
        state.bepinex_installed,
        &state.active_profile,
    ))
}

/// Set the game path manually (e.g., from a directory picker).
#[tauri::command]
pub async fn set_game_path(
    path: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<GameStatus> {
    info!("Command: set_game_path({})", path);
    let _operation = crate::lock_operation(&state)?;

    crate::services::launcher::ensure_game_stopped()?;
    let path_buf = game_detector::normalize_game_path(std::path::Path::new(&path))?;

    let previous = {
        let s = state
            .lock()
            .map_err(|e| crate::error::AppError::GameNotFound(e.to_string()))?;
        (s.game_path.clone(), s.active_profile.clone())
    };
    if let Some(old_path) = &previous.0 {
        if old_path != &path_buf {
            profile_manager::save_game_state_to_profile(
                &previous.1,
                &game_detector::get_valheim_root(old_path),
            )?;
        }
    }

    let game_root = game_detector::get_valheim_root(&path_buf);
    let bepinex_installed =
        crate::services::bepinex_installer::check_bepinex_status(&game_root).installed;

    let active_profile = profile_manager::initialize_game_profile(&game_root)?;
    game_detector::save_game_path(&path_buf)?;

    let mut state = state.lock().map_err(|e| {
        crate::error::AppError::GameNotFound(format!("Failed to lock state: {}", e))
    })?;
    state.game_path = Some(path_buf.clone());
    state.bepinex_installed = bepinex_installed;
    state.active_profile = active_profile;
    state.thunderstore_cache = None;
    state.cache_updated_at = None;

    let status = game_detector::get_game_status_info(
        &Some(path_buf),
        bepinex_installed,
        &state.active_profile,
    );
    Ok(status)
}
