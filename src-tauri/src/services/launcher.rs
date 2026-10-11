use std::path::{Path, PathBuf};
use std::process::Command;

use tracing::info;

use crate::error::{AppError, AppResult};

pub fn is_game_running() -> AppResult<bool> {
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("pgrep").args(["-x", "Valheim"]).output()?;
        match output.status.code() {
            Some(0) => Ok(true),
            Some(1) => Ok(false),
            _ => Err(AppError::Mod(
                "Could not check whether Valheim is running.".into(),
            )),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(false)
    }
}

pub fn ensure_game_stopped() -> AppResult<()> {
    if is_game_running()? {
        return Err(AppError::Mod(
            "Quit Valheim before changing mods, profiles or compatibility patches.".into(),
        ));
    }
    Ok(())
}

/// Check if running on Apple Silicon.
pub fn is_apple_silicon() -> bool {
    let output = Command::new("sysctl")
        .arg("-n")
        .arg("hw.optional.arm64")
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.trim() == "1"
        }
        Err(_) => false,
    }
}

/// Read the CFBundleExecutable from the app's Info.plist.
pub fn get_bundle_executable(app_path: &Path) -> AppResult<String> {
    let info_plist = app_path.join("Contents/Info.plist");

    if !info_plist.exists() {
        return Err(AppError::GameNotFound(
            "Info.plist not found in app bundle".to_string(),
        ));
    }

    match plist::Value::from_file(&info_plist) {
        Ok(plist::Value::Dictionary(dict)) => {
            if let Some(plist::Value::String(executable)) = dict.get("CFBundleExecutable") {
                Ok(executable.clone())
            } else {
                Err(AppError::GameNotFound(
                    "CFBundleExecutable not found in Info.plist".to_string(),
                ))
            }
        }
        _ => Err(AppError::GameNotFound(
            "Failed to parse Info.plist".to_string(),
        )),
    }
}

/// Launch Valheim with BepInEx mod loader.
/// Replicates the exact method used by the working "Valheim Modded.app":
/// Opens Terminal.app and runs run_bepinex.sh there, creating a completely
/// independent process outside the Tauri app's process tree.
pub fn launch_modded(_app_path: &Path, game_root: &Path) -> AppResult<()> {
    info!("Launching Valheim with BepInEx...");

    // Find doorstop library (may be in root or doorstop_libs/)
    let doorstop = find_doorstop_lib(game_root)
        .or_else(|| find_doorstop_lib(&game_root.join("doorstop_libs")))
        .ok_or_else(|| {
            AppError::GameNotFound(
                "doorstop library not found. Please reinstall BepInEx.".to_string(),
            )
        })?;

    // Remove quarantine from doorstop library
    let _ = Command::new("xattr")
        .args(["-dr", "com.apple.quarantine"])
        .arg(&doorstop)
        .output();

    // Verified working method: arch -x86_64 + env + direct DYLD injection
    // This bypasses run_bepinex.sh (which has arm64 conflict) and injects directly.
    let executable_name = get_bundle_executable(&game_root.join("valheim.app"))?;
    let executable = game_root
        .join("valheim.app/Contents/MacOS")
        .join(&executable_name);
    let preloader = game_root.join("BepInEx/core/BepInEx.Preloader.dll");

    if !executable.exists() || !preloader.exists() {
        return Err(AppError::GameNotFound(format!(
            "Missing files: exec={} preloader={}",
            executable.exists(),
            preloader.exists()
        )));
    }

    // Write launcher script for Terminal.app (completely independent process)
    let launcher_script = game_root.join(".vmm_launch.sh");
    let script_content = build_launch_script(game_root, &preloader, &doorstop, &executable);
    std::fs::write(&launcher_script, &script_content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&launcher_script)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&launcher_script, perms)?;
    }

    Command::new("open")
        .arg("-a")
        .arg("Terminal")
        .arg(&launcher_script)
        .spawn()
        .map_err(|e| AppError::GameNotFound(format!("Failed to launch game: {}", e)))?;
    Ok(())
}

fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
}

fn build_launch_script(
    game_root: &Path,
    preloader: &Path,
    doorstop: &Path,
    executable: &Path,
) -> String {
    // Valheim reads its app id from SteamAppId or steam_appid.txt. Without either,
    // Steam init fails ("Using steam APPID:0") and the modded game quits after loading.
    format!(
        r#"#!/bin/bash
set -e
cd {game_root}
open /Applications/Steam.app
arch -x86_64 env \
  SteamAppId=892970 \
  SteamGameId=892970 \
  DOORSTOP_ENABLED=1 \
  DOORSTOP_TARGET_ASSEMBLY={preloader} \
  DYLD_LIBRARY_PATH={game_root} \
  DYLD_INSERT_LIBRARIES={doorstop} \
  {executable} -console
"#,
        game_root = shell_quote(game_root),
        preloader = shell_quote(preloader),
        doorstop = shell_quote(doorstop),
        executable = shell_quote(executable),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn console_flag_and_apostrophe_paths_survive_script_generation() {
        let p = Path::new("/Volumes/Alice's Games/Valheim");
        let script = build_launch_script(p, p, p, p);
        assert!(script.contains("'\\''"));
        assert!(script.contains(" -console\n"));
        assert!(script.contains("arch -x86_64 env"));
        assert_eq!(shell_quote(Path::new("/tmp/$value")), "'/tmp/$value'");
    }

    #[test]
    fn launch_script_passes_steam_app_id() {
        let p = Path::new("/Games/Valheim");
        let script = build_launch_script(p, p, p, p);
        assert!(script.contains("  SteamAppId=892970 \\\n"));
        assert!(script.contains("  SteamGameId=892970 \\\n"));
    }
}

/// Launch Valheim vanilla (without mods) via Steam.
pub fn launch_vanilla() -> AppResult<()> {
    info!("Launching Valheim vanilla via Steam...");

    Command::new("open")
        .arg("steam://rungameid/892970")
        .spawn()
        .map_err(|e| AppError::GameNotFound(format!("Failed to launch Steam: {}", e)))?;

    info!("Valheim vanilla launch initiated");
    Ok(())
}

/// Find the doorstop library in a directory.
fn find_doorstop_lib(dir: &Path) -> Option<PathBuf> {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.starts_with("libdoorstop") && name_str.ends_with(".dylib") {
                    return Some(path);
                }
            }
        }
    }
    None
}
