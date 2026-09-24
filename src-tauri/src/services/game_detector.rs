use std::path::PathBuf;

use regex::Regex;
use tracing::{debug, info, warn};

use crate::error::{AppError, AppResult};

const VALHEIM_APP_ID: &str = "892970";
const VALHEIM_BUNDLE_ID: &str = "com.coffeestain.valheim-steam";

/// Detect the Valheim installation on macOS by scanning Steam library folders.
pub fn detect_valheim() -> AppResult<PathBuf> {
    info!("Detecting Valheim installation...");
    let saved = super::thunderstore_client::get_app_data_dir().join("game-path.json");
    if saved.exists() {
        let path: PathBuf = serde_json::from_slice(&std::fs::read(saved)?)?;
        return normalize_game_path(&path).map_err(|_| AppError::GameNotFound(
            "The saved Valheim location is unavailable. Connect its drive or select a new location; no other installation was selected automatically.".into()));
    }

    // 1. Try parsing Steam libraryfolders.vdf for all library paths
    let library_paths = get_steam_library_paths();

    for lib_path in &library_paths {
        debug!("Checking Steam library: {}", lib_path.display());

        // Check for Valheim app manifest
        let manifest = lib_path
            .join("steamapps")
            .join(format!("appmanifest_{}.acf", VALHEIM_APP_ID));
        if manifest.exists() {
            let valheim_path = lib_path
                .join("steamapps")
                .join("common")
                .join("Valheim")
                .join("valheim.app");

            if valheim_path.exists() {
                if validate_valheim_app(&valheim_path) {
                    info!("Found Valheim at: {}", valheim_path.display());
                    return Ok(valheim_path);
                } else {
                    warn!(
                        "Found valheim.app at {} but bundle ID doesn't match",
                        valheim_path.display()
                    );
                }
            }
        }
    }

    // 2. Try default Steam location directly
    let default_path = get_default_valheim_path();
    if default_path.exists() {
        if validate_valheim_app(&default_path) {
            info!("Found Valheim at default path: {}", default_path.display());
            return Ok(default_path);
        }
    }

    // 3. Try without .app extension (some installs have just the directory)
    for lib_path in &library_paths {
        let valheim_dir = lib_path.join("steamapps").join("common").join("Valheim");
        if valheim_dir.exists() && valheim_dir.is_dir() {
            // Check if there's a valheim.app inside
            let app_path = valheim_dir.join("valheim.app");
            if normalize_game_path(&app_path).is_ok() {
                info!("Found Valheim app bundle at: {}", app_path.display());
                return Ok(app_path);
            }
        }
    }

    Err(AppError::GameNotFound(
        "Could not find Valheim installation. Please ensure Valheim is installed via Steam."
            .to_string(),
    ))
}

/// Accept only a real Valheim app, its containing folder, or a Steam library.
/// Never accept an arbitrary directory as a BepInEx installation target.
pub fn normalize_game_path(path: &std::path::Path) -> AppResult<PathBuf> {
    let candidates = [
        path.to_path_buf(),
        path.join("valheim.app"),
        path.join("Valheim.app"),
        path.join("steamapps/common/Valheim/valheim.app"),
    ];
    for candidate in candidates {
        if candidate.extension().is_none_or(|ext| ext != "app") {
            continue;
        }
        let plist = candidate.join("Contents/Info.plist");
        if let Ok(plist::Value::Dictionary(dict)) = plist::Value::from_file(plist) {
            let id = dict
                .get("CFBundleIdentifier")
                .and_then(plist::Value::as_string);
            let executable = dict
                .get("CFBundleExecutable")
                .and_then(plist::Value::as_string);
            if id == Some(VALHEIM_BUNDLE_ID)
                && executable.is_some_and(|exe| {
                    !exe.contains('/')
                        && !exe.contains('\\')
                        && candidate.join("Contents/MacOS").join(exe).is_file()
                })
            {
                return Ok(std::fs::canonicalize(candidate)?);
            }
        }
    }
    Err(AppError::GameNotFound("Select valheim.app, its containing Valheim folder, or its Steam library. The selected location is not a valid Steam Valheim installation.".into()))
}

pub fn save_game_path(path: &std::path::Path) -> AppResult<()> {
    super::compatibility::atomic_write(
        &super::thunderstore_client::get_app_data_dir().join("game-path.json"),
        &serde_json::to_vec(path)?,
    )
}

/// Get the root directory containing the valheim.app (or the game dir itself).
/// This is where BepInEx files go.
pub fn get_valheim_root(game_path: &PathBuf) -> PathBuf {
    if game_path.extension().map(|e| e == "app").unwrap_or(false) {
        // game_path is valheim.app, parent is the Valheim root
        game_path.parent().unwrap_or(game_path).to_path_buf()
    } else {
        game_path.clone()
    }
}

/// Parse Steam's libraryfolders.vdf to find all Steam library paths.
fn get_steam_library_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Default Steam path
    if let Some(home) = dirs::home_dir() {
        let default_steam = home.join("Library/Application Support/Steam");
        if default_steam.exists() {
            paths.push(default_steam.clone());
        }

        let vdf_path = default_steam.join("steamapps/libraryfolders.vdf");
        if vdf_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&vdf_path) {
                let parsed = parse_vdf_paths(&content);
                for p in parsed {
                    let path = PathBuf::from(p);
                    if path.exists() && !paths.contains(&path) {
                        paths.push(path);
                    }
                }
            }
        }
    }

    paths
}

/// Simple regex-based parser for Valve's VDF format to extract "path" values.
fn parse_vdf_paths(content: &str) -> Vec<String> {
    let mut result = Vec::new();

    // Match lines like: "path"		"/path/to/library"
    let re = Regex::new(r#""path"\s+"([^"]+)""#).unwrap();
    for cap in re.captures_iter(content) {
        if let Some(m) = cap.get(1) {
            result.push(m.as_str().to_string());
        }
    }

    result
}

/// Default Valheim path on macOS
fn get_default_valheim_path() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/Users"));
    home.join("Library/Application Support/Steam/steamapps/common/Valheim/valheim.app")
}

/// Validate the app bundle by checking Info.plist for the correct CFBundleIdentifier.
fn validate_valheim_app(app_path: &PathBuf) -> bool {
    normalize_game_path(app_path).is_ok()
}

/// Get the game status information for the frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct GameStatus {
    pub installed: bool,
    pub game_path: Option<String>,
    pub bepinex_installed: bool,
    pub active_profile: String,
}

pub fn get_game_status_info(
    game_path: &Option<PathBuf>,
    bepinex_installed: bool,
    active_profile: &str,
) -> GameStatus {
    match game_path {
        Some(path) => GameStatus {
            installed: true,
            game_path: Some(path.display().to_string()),
            bepinex_installed,
            active_profile: active_profile.to_string(),
        },
        None => GameStatus {
            installed: false,
            game_path: None,
            bepinex_installed: false,
            active_profile: "Default".to_string(),
        },
    }
}

/// Try to read the game version from Info.plist
fn read_game_version(app_path: &PathBuf) -> Option<String> {
    let info_plist = app_path.join("Contents/Info.plist");
    if !info_plist.exists() {
        return None;
    }

    match plist::Value::from_file(&info_plist) {
        Ok(plist::Value::Dictionary(dict)) => {
            if let Some(plist::Value::String(version)) = dict.get("CFBundleShortVersionString") {
                Some(version.clone())
            } else if let Some(plist::Value::String(version)) = dict.get("CFBundleVersion") {
                Some(version.clone())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_external_paths_and_rejects_unrelated_folders() {
        let drive = tempfile::tempdir().unwrap();
        let root = drive
            .path()
            .join("External Library/steamapps/common/Valheim");
        let app = root.join("valheim.app");
        std::fs::create_dir_all(app.join("Contents/MacOS")).unwrap();
        assert!(normalize_game_path(&root).is_err());
        std::fs::write(app.join("Contents/MacOS/Valheim"), b"fixture").unwrap();
        let mut dict = plist::Dictionary::new();
        dict.insert("CFBundleIdentifier".into(), VALHEIM_BUNDLE_ID.into());
        dict.insert("CFBundleExecutable".into(), "Valheim".into());
        plist::Value::Dictionary(dict)
            .to_file_xml(app.join("Contents/Info.plist"))
            .unwrap();
        let canonical = std::fs::canonicalize(&app).unwrap();
        assert_eq!(normalize_game_path(&app).unwrap(), canonical);
        assert_eq!(normalize_game_path(&root).unwrap(), canonical);
        assert_eq!(
            normalize_game_path(&drive.path().join("External Library")).unwrap(),
            canonical
        );
        assert!(normalize_game_path(drive.path()).is_err());
        assert_eq!(
            get_valheim_root(&canonical),
            std::fs::canonicalize(root).unwrap()
        );
    }
}
