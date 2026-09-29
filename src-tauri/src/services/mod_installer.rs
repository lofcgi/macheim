use std::path::Path;

use tracing::{debug, info};

use crate::error::{AppError, AppResult};
use crate::models::InstalledMod;
use crate::services::gatekeeper;
use crate::services::thunderstore_client;

/// Install a mod from a ZIP downloaded from Thunderstore.
/// Returns the InstalledMod metadata.
pub async fn install_mod(
    author: &str,
    name: &str,
    version: &str,
    download_url: &str,
    description: &str,
    icon: &str,
    dependencies: &[String],
    game_root: &Path,
) -> AppResult<InstalledMod> {
    let full_name = format!("{}-{}", author, name);
    info!("Installing mod: {} v{}", full_name, version);

    // Download the mod ZIP
    let zip_bytes = thunderstore_client::download_mod(download_url).await?;

    // Install from bytes
    install_mod_from_bytes(
        author,
        name,
        version,
        description,
        icon,
        dependencies,
        &zip_bytes,
        game_root,
    )
}

/// Install a mod from ZIP bytes (used by both direct install and dependency install).
pub fn install_mod_from_bytes(
    author: &str,
    name: &str,
    version: &str,
    description: &str,
    icon: &str,
    dependencies: &[String],
    zip_bytes: &[u8],
    game_root: &Path,
) -> AppResult<InstalledMod> {
    let full_name = format!("{}-{}", author, name);
    crate::services::profile_manager::validate_name(&full_name)?;
    crate::services::compatibility::reject_symlink_ancestors(&game_root.join("BepInEx"))?;

    // Extract ZIP to temp directory for analysis
    let temp_dir = tempfile::tempdir()?;
    extract_zip(zip_bytes, temp_dir.path())?;

    // Analyze the mod structure and copy to appropriate locations
    let bepinex_dir = game_root.join("BepInEx");
    install_mod_files(temp_dir.path(), &full_name, &bepinex_dir)?;

    // Remove quarantine from any .dylib files
    let plugins_dir = bepinex_dir.join("plugins").join(&full_name);
    if plugins_dir.exists() {
        gatekeeper::remove_quarantine_from_dylibs(&plugins_dir)?;
    }

    let installed = InstalledMod {
        source: None,
        full_name,
        author: author.to_string(),
        name: name.to_string(),
        version: version.to_string(),
        description: description.to_string(),
        enabled: true,
        dependencies: dependencies.to_vec(),
        installed_at: chrono::Utc::now().to_rfc3339(),
        icon: icon.to_string(),
    };

    info!(
        "Mod installed: {} v{}",
        installed.full_name, installed.version
    );
    Ok(installed)
}

/// Analyze mod structure and copy files to the correct BepInEx directories.
fn install_mod_files(extracted_dir: &Path, mod_name: &str, bepinex_dir: &Path) -> AppResult<()> {
    let plugins_dir = bepinex_dir.join("plugins").join(mod_name);
    let patchers_dir = bepinex_dir.join("patchers");
    let config_dir = bepinex_dir.join("config");

    // Check for standard mod structure
    let has_plugins = extracted_dir.join("plugins").exists();
    let has_patchers = extracted_dir.join("patchers").exists();
    let has_config = extracted_dir.join("config").exists();

    if has_plugins || has_patchers || has_config {
        // Standard Thunderstore mod structure
        if has_plugins {
            std::fs::create_dir_all(&plugins_dir)?;
            copy_dir_contents(&extracted_dir.join("plugins"), &plugins_dir)?;
            debug!("Copied plugins/ to {}", plugins_dir.display());
        }

        if has_patchers {
            std::fs::create_dir_all(&patchers_dir)?;
            copy_dir_contents(&extracted_dir.join("patchers"), &patchers_dir)?;
            debug!("Copied patchers/ to {}", patchers_dir.display());
        }

        if has_config {
            std::fs::create_dir_all(&config_dir)?;
            copy_config_defaults(&extracted_dir.join("config"), &config_dir)?;
            debug!("Copied config/ to {}", config_dir.display());
        }
    }

    // Some packages (including Therzie's mods) combine config/ with root DLLs.
    // Route those files independently of the standard directories above.
    let has_root_dlls = has_dll_files(extracted_dir);

    if has_root_dlls {
        // Treat root DLLs as plugins
        std::fs::create_dir_all(&plugins_dir)?;
        copy_files_by_extension(extracted_dir, &plugins_dir, &["dll", "dylib", "so"])?;
        debug!("Copied root DLLs to {}", plugins_dir.display());
    }

    // Standard directories are excluded by this helper; keep root assets too.
    copy_non_metadata_files(extracted_dir, &plugins_dir)?;

    Ok(())
}

/// Uninstall a mod by removing its files.
pub fn uninstall_mod(mod_full_name: &str, game_root: &Path) -> AppResult<()> {
    crate::services::profile_manager::validate_name(mod_full_name)?;
    info!("Uninstalling mod: {}", mod_full_name);

    let bepinex_dir = game_root.join("BepInEx");

    // Remove from plugins
    let plugins_dir = bepinex_dir.join("plugins").join(mod_full_name);
    if plugins_dir.exists() {
        crate::services::compatibility::reject_symlink_ancestors(&plugins_dir)?;
        if plugins_dir.is_file() {
            std::fs::remove_file(&plugins_dir)?;
        } else {
            std::fs::remove_dir_all(&plugins_dir)?;
        }
        debug!("Removed plugins: {}", plugins_dir.display());
    }

    // Also check plugins_disabled
    let disabled_dir = bepinex_dir.join("plugins_disabled").join(mod_full_name);
    if disabled_dir.exists() {
        crate::services::compatibility::reject_symlink_ancestors(&disabled_dir)?;
        if disabled_dir.is_file() {
            std::fs::remove_file(&disabled_dir)?;
        } else {
            std::fs::remove_dir_all(&disabled_dir)?;
        }
        debug!("Removed disabled plugins: {}", disabled_dir.display());
    }

    info!("Mod uninstalled: {}", mod_full_name);
    Ok(())
}

/// Enable or disable a mod by moving it between plugins/ and plugins_disabled/.
pub fn toggle_mod(mod_full_name: &str, enable: bool, game_root: &Path) -> AppResult<bool> {
    crate::services::profile_manager::validate_name(mod_full_name)?;
    let bepinex_dir = game_root.join("BepInEx");
    let plugins_dir = bepinex_dir.join("plugins").join(mod_full_name);
    let disabled_dir = bepinex_dir.join("plugins_disabled").join(mod_full_name);

    if enable {
        // Move from disabled to plugins
        if disabled_dir.exists() {
            std::fs::create_dir_all(bepinex_dir.join("plugins"))?;
            move_dir(&disabled_dir, &plugins_dir)?;
            info!("Enabled mod: {}", mod_full_name);
            Ok(true)
        } else if plugins_dir.exists() {
            // Already enabled
            Ok(true)
        } else {
            Err(AppError::Mod(format!(
                "Mod '{}' files not found",
                mod_full_name
            )))
        }
    } else {
        // Move from plugins to disabled
        if plugins_dir.exists() {
            std::fs::create_dir_all(bepinex_dir.join("plugins_disabled"))?;
            move_dir(&plugins_dir, &disabled_dir)?;
            info!("Disabled mod: {}", mod_full_name);
            Ok(false)
        } else if disabled_dir.exists() {
            // Already disabled
            Ok(false)
        } else {
            Err(AppError::Mod(format!(
                "Mod '{}' files not found",
                mod_full_name
            )))
        }
    }
}

/// Get list of installed mods by scanning the BepInEx/plugins directory.
pub fn scan_installed_mods(game_root: &Path) -> AppResult<Vec<String>> {
    let plugins_dir = game_root.join("BepInEx").join("plugins");
    let mut mods = Vec::new();

    if plugins_dir.exists() {
        for entry in std::fs::read_dir(&plugins_dir)? {
            let entry = entry?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip internal BepInEx directories
                if !name.starts_with('.') {
                    mods.push(name);
                }
            }
        }
    }

    Ok(mods)
}

// --- Helper functions ---

fn extract_zip(zip_bytes: &[u8], target: &Path) -> AppResult<()> {
    let cursor = std::io::Cursor::new(zip_bytes);
    let mut archive = zip::ZipArchive::new(cursor)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let outpath = target.join(file.mangled_name());

        if file.is_dir() {
            std::fs::create_dir_all(&outpath)?;
        } else {
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut outfile = std::fs::File::create(&outpath)?;
            std::io::copy(&mut file, &mut outfile)?;
        }
    }

    Ok(())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> AppResult<()> {
    crate::services::compatibility::reject_symlink_ancestors(dst)?;
    if !src.is_dir() {
        return Ok(());
    }

    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        crate::services::compatibility::reject_symlink_ancestors(&dst_path)?;

        if src_path.is_dir() {
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn move_dir(src: &Path, dst: &Path) -> AppResult<()> {
    crate::services::compatibility::reject_symlink_ancestors(src)?;
    crate::services::compatibility::reject_symlink_ancestors(dst)?;
    if dst.exists() {
        return Err(AppError::Mod(
            "Both enabled and disabled copies exist. Back them up and resolve the conflict first."
                .into(),
        ));
    }
    // Try rename first (fast, same filesystem)
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }

    // Fallback: copy then remove
    if src.is_file() {
        std::fs::copy(src, dst)?;
        std::fs::remove_file(src)?;
    } else {
        copy_dir_contents(src, dst)?;
        std::fs::remove_dir_all(src)?;
    }
    Ok(())
}

fn copy_config_defaults(src: &Path, dst: &Path) -> AppResult<()> {
    crate::services::compatibility::reject_symlink_ancestors(dst)?;
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        crate::services::compatibility::reject_symlink_ancestors(&dest)?;
        if entry.path().is_dir() {
            copy_config_defaults(&entry.path(), &dest)?;
        } else if !dest.exists() {
            std::fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

fn has_dll_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(ext) = entry.path().extension() {
                if ext == "dll" || ext == "dylib" || ext == "so" {
                    return true;
                }
            }
        }
    }
    false
}

fn copy_files_by_extension(src: &Path, dst: &Path, extensions: &[&str]) -> AppResult<()> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if extensions.iter().any(|e| ext == *e) {
                    let dst_file = dst.join(entry.file_name());
                    crate::services::compatibility::reject_symlink_ancestors(&dst_file)?;
                    std::fs::copy(&path, &dst_file)?;
                }
            }
        }
    }

    Ok(())
}

fn copy_non_metadata_files(src: &Path, dst: &Path) -> AppResult<()> {
    let skip_files = [
        "manifest.json",
        "icon.png",
        "README.md",
        "CHANGELOG.md",
        "LICENSE",
    ];

    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();

        if path.is_file() && !skip_files.contains(&name.as_str()) {
            let dst_file = dst.join(&name);
            if !dst_file.exists() {
                std::fs::copy(&path, &dst_file)?;
            }
        } else if path.is_dir() {
            let dir_name = name.to_lowercase();
            // Skip standard Thunderstore directories already handled
            if dir_name != "plugins" && dir_name != "patchers" && dir_name != "config" {
                let dst_subdir = dst.join(&name);
                copy_dir_contents(&path, &dst_subdir)?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reinstall_preserves_existing_config_values() {
        let source = tempfile::tempdir().unwrap();
        let game = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("config")).unwrap();
        std::fs::write(source.path().join("config/custom.cfg"), b"new defaults").unwrap();
        std::fs::create_dir(game.path().join("config")).unwrap();
        std::fs::write(game.path().join("config/custom.cfg"), b"user edits").unwrap();
        install_mod_files(source.path(), "Test-Mod", game.path()).unwrap();
        assert_eq!(
            std::fs::read(game.path().join("config/custom.cfg")).unwrap(),
            b"user edits"
        );
    }

    #[test]
    fn toggles_loose_manual_dll_without_overwriting_conflicts() {
        let game = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(game.path().join("BepInEx/plugins")).unwrap();
        std::fs::write(game.path().join("BepInEx/plugins/Loose.dll"), b"manual").unwrap();
        assert!(!toggle_mod("Loose.dll", false, game.path()).unwrap());
        assert!(toggle_mod("Loose.dll", true, game.path()).unwrap());
        std::fs::write(
            game.path().join("BepInEx/plugins_disabled/Loose.dll"),
            b"conflict",
        )
        .unwrap();
        assert!(toggle_mod("Loose.dll", false, game.path()).is_err());
        assert_eq!(
            std::fs::read(game.path().join("BepInEx/plugins/Loose.dll")).unwrap(),
            b"manual"
        );
    }

    #[test]
    fn installs_root_plugin_and_assets_alongside_config() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        let source = source.path();
        std::fs::create_dir(source.join("config")).unwrap();
        std::fs::write(source.join("config/translation.yml"), "name: test").unwrap();
        std::fs::write(source.join("Wizardry.dll"), b"plugin").unwrap();
        std::fs::write(source.join("wizardry.bundle"), b"assets").unwrap();
        std::fs::write(source.join("manifest.json"), "{}").unwrap();

        install_mod_files(source, "Therzie-Wizardry", destination.path()).unwrap();

        let plugin = destination.path().join("plugins/Therzie-Wizardry");
        assert_eq!(
            std::fs::read(plugin.join("Wizardry.dll")).unwrap(),
            b"plugin"
        );
        assert_eq!(
            std::fs::read(plugin.join("wizardry.bundle")).unwrap(),
            b"assets"
        );
        assert!(destination.path().join("config/translation.yml").is_file());
        assert!(!plugin.join("config").exists());
        assert!(!plugin.join("manifest.json").exists());
    }

    #[test]
    fn keeps_standard_plugin_and_patcher_routes() {
        let source = tempfile::tempdir().unwrap();
        let destination = tempfile::tempdir().unwrap();
        for dir in ["plugins", "patchers", "config"] {
            std::fs::create_dir(source.path().join(dir)).unwrap();
        }
        std::fs::write(source.path().join("plugins/Main.dll"), b"main").unwrap();
        std::fs::write(source.path().join("patchers/Patcher.dll"), b"patcher").unwrap();
        std::fs::write(source.path().join("config/test.cfg"), b"config").unwrap();
        std::fs::write(source.path().join("Helper.dll"), b"helper").unwrap();

        install_mod_files(source.path(), "Test-Mod", destination.path()).unwrap();

        assert!(destination
            .path()
            .join("plugins/Test-Mod/Main.dll")
            .is_file());
        assert!(destination
            .path()
            .join("plugins/Test-Mod/Helper.dll")
            .is_file());
        assert!(destination.path().join("patchers/Patcher.dll").is_file());
        assert!(destination.path().join("config/test.cfg").is_file());
        assert!(!destination
            .path()
            .join("plugins/Test-Mod/patchers")
            .exists());
    }
}
