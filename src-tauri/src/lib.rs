use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod commands;
pub mod error;
pub mod models;
pub mod services;

use models::ThunderstorePackage;

/// Migrate app data from old directory name to new one.
fn migrate_app_data_dir() {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return,
    };
    let old = home.join("Library/Application Support/com.valheim-mod-manager");
    let new = home.join("Library/Application Support/com.macheim");
    if old.exists() && !new.exists() {
        if let Err(e) = std::fs::rename(&old, &new) {
            tracing::warn!("Failed to migrate app data: {}", e);
        } else {
            tracing::info!("Migrated app data from {:?} to {:?}", old, new);
        }
    }
}

/// Global application state shared across Tauri commands.
pub struct AppState {
    pub operation_lock: Arc<tokio::sync::Mutex<()>>,
    /// Path to the Valheim app bundle (e.g., .../Valheim/valheim.app)
    pub game_path: Option<PathBuf>,
    /// Whether BepInEx is currently installed
    pub bepinex_installed: bool,
    /// Name of the currently active mod profile
    pub active_profile: String,
    /// Cached Thunderstore package data
    pub thunderstore_cache: Option<Vec<ThunderstorePackage>>,
    /// When the Thunderstore cache was last updated
    pub cache_updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            operation_lock: Arc::new(tokio::sync::Mutex::new(())),
            game_path: None,
            bepinex_installed: false,
            active_profile: "Default".to_string(),
            thunderstore_cache: None,
            cache_updated_at: None,
        }
    }
}

/// Serialize filesystem mutations across async commands; never hold the state lock over I/O awaits.
pub fn lock_operation(
    state: &Mutex<AppState>,
) -> error::AppResult<tokio::sync::OwnedMutexGuard<()>> {
    let lock = state
        .lock()
        .map_err(|e| error::AppError::Mod(e.to_string()))?
        .operation_lock
        .clone();
    lock.try_lock_owned().map_err(|_| {
        error::AppError::Mod(
            "Another operation is in progress. Please wait for it to finish.".into(),
        )
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing for structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Migrate app data from old name if needed
    migrate_app_data_dir();

    tracing::info!("Starting Macheim");

    // Ensure the default profile exists
    if let Err(e) = services::profile_manager::ensure_default_profile() {
        tracing::warn!("Failed to create default profile: {}", e);
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Mutex::new(AppState::default()))
        .invoke_handler(tauri::generate_handler![
            // Game detection
            commands::game::detect_game,
            commands::game::get_game_status,
            commands::game::set_game_path,
            // BepInEx management
            commands::bepinex::install_bepinex,
            commands::bepinex::get_bepinex_status,
            commands::bepinex::uninstall_bepinex,
            // Thunderstore
            commands::thunderstore::fetch_packages,
            commands::thunderstore::search_packages,
            commands::thunderstore::get_package_details,
            services::download_settings::get_download_cdn,
            services::download_settings::set_download_cdn,
            // Mod management
            commands::mods::install_mod,
            commands::mods::uninstall_mod,
            commands::mods::toggle_mod,
            commands::mods::get_installed_mods,
            commands::mods::install_modpack,
            commands::mods::sync_mods,
            commands::mods::list_unmanaged_mods,
            commands::updates::check_mod_updates,
            commands::updates::change_mod_version,
            commands::updates::update_mods,
            // Profiles
            commands::profiles::list_profiles,
            commands::profiles::create_profile,
            commands::profiles::switch_profile,
            commands::profiles::delete_profile,
            commands::profiles::clone_profile,
            commands::profiles::export_profile,
            commands::profiles::import_profile,
            commands::profiles::get_active_profile,
            // Config editor
            commands::config::get_config_files,
            commands::config::get_config,
            commands::config::save_config,
            // Backups
            commands::backup::create_backup,
            commands::backup::list_backups,
            commands::backup::restore_backup,
            // Launch
            commands::launch::launch_modded,
            commands::launch::launch_vanilla,
            commands::compatibility::get_compatibility,
            commands::compatibility::apply_compatibility,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
