use std::sync::Mutex;

use tracing::info;

use crate::error::{AppError, AppResult};
use crate::models::Profile;
use crate::services::{game_detector, profile_manager};
use crate::AppState;

/// List all profiles.
#[tauri::command]
pub async fn list_profiles(_state: tauri::State<'_, Mutex<AppState>>) -> AppResult<Vec<Profile>> {
    let profiles = profile_manager::list_profiles()?;
    Ok(profiles)
}

/// Create a new profile.
#[tauri::command]
pub async fn create_profile(
    name: String,
    description: Option<String>,
    catalog_source: Option<crate::models::profile::CatalogSource>,
    _state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Profile> {
    info!("Command: create_profile({})", name);

    let desc = description.unwrap_or_default();
    let mut profile = profile_manager::create_profile(&name, &desc)?;
    profile.catalog_source = catalog_source.unwrap_or_default();
    profile_manager::save_profile(&profile)?;
    Ok(profile)
}

/// Switch to a different profile.
#[tauri::command]
pub async fn switch_profile(
    name: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Profile> {
    info!("Command: switch_profile({})", name);
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;
    profile_manager::load_profile(&name)?;
    let current_profile = state
        .lock()
        .map_err(|e| AppError::Profile(e.to_string()))?
        .active_profile
        .clone();
    if current_profile == name {
        return profile_manager::load_profile(&name);
    }

    let game_path = {
        let state = state
            .lock()
            .map_err(|e| AppError::Profile(format!("Failed to lock state: {}", e)))?;
        state.game_path.clone()
    };

    if let Some(game_path) = &game_path {
        let game_root = game_detector::get_valheim_root(game_path);

        // Save current profile state first
        {
            let state = state
                .lock()
                .map_err(|e| AppError::Profile(format!("Failed to lock state: {}", e)))?;
            let current_profile = state.active_profile.clone();
            profile_manager::save_game_state_to_profile(&current_profile, &game_root)?;
        }

        // Switch to new profile
        profile_manager::switch_profile(&name, &game_root)?;
        let result = (|| {
            crate::services::compatibility::reconcile(
                &profile_manager::load_profile(&name)?,
                &game_root,
            )?;
            profile_manager::set_active_profile(&name, &game_root)
        })();
        if let Err(error) = result {
            profile_manager::switch_profile(&current_profile, &game_root)?;
            profile_manager::set_active_profile(&current_profile, &game_root)?;
            return Err(error);
        }
    }

    let profile = profile_manager::load_profile(&name)?;

    // Update state
    let mut state = state
        .lock()
        .map_err(|e| AppError::Profile(format!("Failed to lock state: {}", e)))?;
    state.active_profile = name;
    state.thunderstore_cache = None;
    state.cache_updated_at = None;

    Ok(profile)
}

/// Delete a profile.
#[tauri::command]
pub async fn delete_profile(
    name: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<()> {
    info!("Command: delete_profile({})", name);

    {
        let state = state
            .lock()
            .map_err(|e| AppError::Profile(format!("Failed to lock state: {}", e)))?;
        if state.active_profile == name {
            return Err(AppError::Profile(
                "Cannot delete the currently active profile. Switch to another profile first."
                    .to_string(),
            ));
        }
    }

    profile_manager::delete_profile(&name)?;
    Ok(())
}

/// Clone a profile.
#[tauri::command]
pub async fn clone_profile(
    source_name: String,
    new_name: String,
    _state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Profile> {
    info!("Command: clone_profile({} -> {})", source_name, new_name);
    let profile = profile_manager::clone_profile(&source_name, &new_name)?;
    Ok(profile)
}

/// Export a profile to JSON string (for saving to a file via the frontend).
#[tauri::command]
pub async fn export_profile(
    name: String,
    _state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<String> {
    info!("Command: export_profile({})", name);
    let json = profile_manager::export_profile(&name)?;
    Ok(json)
}

/// Import a profile from a JSON string.
#[tauri::command]
pub async fn import_profile(
    json: String,
    new_name: Option<String>,
    _state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Profile> {
    info!("Command: import_profile");
    let profile = profile_manager::import_profile(&json, new_name.as_deref())?;
    Ok(profile)
}

/// Get the currently active profile name.
#[tauri::command]
pub async fn get_active_profile(state: tauri::State<'_, Mutex<AppState>>) -> AppResult<String> {
    let state = state
        .lock()
        .map_err(|e| AppError::Profile(format!("Failed to lock state: {}", e)))?;
    Ok(state.active_profile.clone())
}
