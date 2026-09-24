use crate::{
    error::{AppError, AppResult},
    models::InstalledMod,
    services::{
        dependency_resolver, game_detector, mod_installer, profile_manager, thunderstore_client,
    },
    AppState,
};
use std::{collections::HashSet, sync::Mutex};

#[derive(serde::Serialize)]
pub struct ModUpdate {
    full_name: String,
    name: String,
    current_version: String,
    latest_version: String,
}

fn stage_package(
    p: &crate::models::ThunderstorePackage,
    v: &crate::models::thunderstore::PackageVersion,
    bytes: &[u8],
    root: &std::path::Path,
    enabled: bool,
) -> AppResult<InstalledMod> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))?;
    let manifest: crate::models::manifest::Manifest =
        serde_json::from_reader(archive.by_name("manifest.json")?)?;
    if manifest.name != p.name || manifest.version_number != v.version_number {
        return Err(AppError::Mod(
            "Downloaded manifest does not match the selected package/version".into(),
        ));
    }
    mod_installer::uninstall_mod(&p.full_name, root)?;
    let mut result = mod_installer::install_mod_from_bytes(
        &p.owner,
        &p.name,
        &v.version_number,
        &v.description,
        &v.icon,
        &v.dependencies,
        bytes,
        root,
    )?;
    if !enabled && root.join("BepInEx/plugins").join(&p.full_name).exists() {
        mod_installer::toggle_mod(&p.full_name, false, root)?;
    }
    result.enabled = enabled;
    Ok(result)
}

#[tauri::command]
pub async fn check_mod_updates(
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Vec<ModUpdate>> {
    let _operation = crate::lock_operation(&state)?;
    let name = state
        .lock()
        .map_err(|e| AppError::Mod(e.to_string()))?
        .active_profile
        .clone();
    let profile = profile_manager::load_profile(&name)?;
    let packages = thunderstore_client::fetch_catalog(profile.catalog_source, true).await?;
    let updates = profile
        .mods
        .iter()
        .filter_map(|m| {
            if m.version == "0.0.0" {
                return None;
            }
            let p = thunderstore_client::find_package(&packages, &m.full_name)?;
            if p.is_deprecated {
                return None;
            }
            let latest = p.versions.first()?;
            let current = semver::Version::parse(&m.version).ok()?;
            let next = semver::Version::parse(&latest.version_number).ok()?;
            (next > current).then(|| ModUpdate {
                full_name: m.full_name.clone(),
                name: m.name.clone(),
                current_version: m.version.clone(),
                latest_version: latest.version_number.clone(),
            })
        })
        .collect();
    state
        .lock()
        .map_err(|e| AppError::Mod(e.to_string()))?
        .thunderstore_cache = Some(packages);
    Ok(updates)
}

/// Stage the complete profile before replacing any live files. Never delete an
/// installed version just because the replacement download returned HTTP 200.
#[tauri::command]
pub async fn change_mod_version(
    full_name: String,
    version: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> AppResult<Vec<InstalledMod>> {
    let _operation = crate::lock_operation(&state)?;
    crate::services::launcher::ensure_game_stopped()?;
    profile_manager::validate_name(&full_name)?;
    if full_name == "denikson-BepInExPack_Valheim" {
        return Err(AppError::Mod(
            "Use the BepInEx installer in setup; the loader is not a regular plugin.".into(),
        ));
    }
    let (path, name, packages) = {
        let s = state.lock().map_err(|e| AppError::Mod(e.to_string()))?;
        (
            s.game_path
                .clone()
                .ok_or_else(|| AppError::Mod("Game path not set".into()))?,
            s.active_profile.clone(),
            s.thunderstore_cache
                .clone()
                .ok_or_else(|| AppError::Mod("Refresh the catalog first".into()))?,
        )
    };
    let root = game_detector::get_valheim_root(&path);
    let mut profile = profile_manager::load_profile(&name)?;
    let original = profile.clone();
    let target = thunderstore_client::find_package(&packages, &full_name)
        .ok_or_else(|| AppError::Mod("Package not in this profile's catalog".into()))?;
    let target_version = target
        .versions
        .iter()
        .find(|v| v.version_number == version)
        .ok_or_else(|| AppError::Mod("Selected version is unavailable".into()))?;
    let mut installed: HashSet<String> = profile.mods.iter().map(|m| m.full_name.clone()).collect();
    installed.insert("denikson-BepInExPack_Valheim".into());
    // Do not silently upgrade a dependency shared with another installed mod.
    for dep in &target_version.dependencies {
        if let Some(required) = crate::models::thunderstore::ParsedDependency::parse(dep) {
            if let Some(current) = profile
                .mods
                .iter()
                .find(|m| m.full_name == required.full_name)
            {
                match (
                    semver::Version::parse(&current.version),
                    semver::Version::parse(&required.version),
                ) {
                    (Ok(a), Ok(b)) if a >= b && current.enabled => {}
                    _ => return Err(AppError::Mod(format!(
                        "Update/enable dependency {} to at least {} first. No files were changed.",
                        required.full_name, required.version
                    ))),
                }
            }
        }
    }
    let deps =
        dependency_resolver::resolve_dependencies(&full_name, &version, &packages, &installed)?;
    let stage = tempfile::tempdir()?;
    profile_manager::replace_bepinex_dirs(&root.join("BepInEx"), &stage.path().join("BepInEx"))?;
    let mut plan: Vec<_> = deps
        .iter()
        .map(|dep| (dep.full_name.clone(), dep.version.clone()))
        .collect();
    plan.push((full_name, version));
    for (id, ver) in plan {
        let p = thunderstore_client::find_package(&packages, &id)
            .ok_or_else(|| AppError::Mod(format!("Missing package {id}")))?;
        let v = p
            .versions
            .iter()
            .find(|v| v.version_number == ver)
            .ok_or_else(|| AppError::Mod(format!("Missing version {ver}")))?;
        let bytes = thunderstore_client::download_mod(&v.download_url).await?;
        let enabled = profile
            .mods
            .iter()
            .find(|m| m.full_name == id)
            .is_none_or(|m| m.enabled);
        let result = stage_package(p, v, &bytes, stage.path(), enabled)?;
        profile.mods.retain(|m| m.full_name != id);
        profile.mods.push(result);
    }
    crate::services::compatibility::reconcile(&profile, stage.path())?;
    let recovery = tempfile::Builder::new()
        .prefix(".macheim-update-recovery-")
        .tempdir_in(&root)?;
    profile_manager::replace_bepinex_dirs(&root.join("BepInEx"), &recovery.path().join("BepInEx"))?;
    std::fs::write(
        recovery.path().join("profile.json"),
        serde_json::to_vec_pretty(&original)?,
    )?;
    profile_manager::replace_bepinex_dirs(&stage.path().join("BepInEx"), &root.join("BepInEx"))?;
    profile.touch();
    if let Err(error) = profile_manager::save_profile(&profile) {
        if let Err(rollback) = profile_manager::replace_bepinex_dirs(
            &recovery.path().join("BepInEx"),
            &root.join("BepInEx"),
        ) {
            let retained = recovery.keep();
            return Err(AppError::Mod(format!(
                "{error}; rollback failed: {rollback}. Recovery retained at {}",
                retained.display()
            )));
        }
        return Err(error);
    }
    Ok(profile.mods)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn package() -> crate::models::ThunderstorePackage {
        serde_json::from_value(serde_json::json!({"name":"Mod","owner":"Team","full_name":"Team-Mod","package_url":"","date_updated":"","is_deprecated":false,"rating_score":0,
          "versions":[{"name":"Mod","full_name":"Team-Mod-2.0.0","version_number":"2.0.0","dependencies":[],"download_url":"","downloads":0,"description":"","icon":"","date_created":""}]})).unwrap()
    }
    #[test]
    fn invalid_download_does_not_remove_old_files() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("BepInEx/plugins/Team-Mod");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("old.dll"), b"original").unwrap();
        let p = package();
        assert!(stage_package(
            &p,
            &p.versions[0],
            b"<html>blocked</html>",
            root.path(),
            true
        )
        .is_err());
        assert_eq!(std::fs::read(folder.join("old.dll")).unwrap(), b"original");
    }
    #[test]
    fn staged_update_preserves_config_disabled_state_and_unrelated_mods() {
        let root = tempfile::tempdir().unwrap();
        for folder in ["plugins/Manual", "plugins_disabled/Team-Mod", "config"] {
            std::fs::create_dir_all(root.path().join("BepInEx").join(folder)).unwrap();
        }
        std::fs::write(
            root.path().join("BepInEx/plugins/Manual/manual.dll"),
            b"manual",
        )
        .unwrap();
        std::fs::write(
            root.path()
                .join("BepInEx/plugins_disabled/Team-Mod/old.dll"),
            b"old",
        )
        .unwrap();
        std::fs::write(
            root.path().join("BepInEx/config/custom.cfg"),
            b"user values",
        )
        .unwrap();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        let options = zip::write::SimpleFileOptions::default();
        for (name, data) in [
            (
                "manifest.json",
                r#"{"name":"Mod","version_number":"2.0.0","description":"","dependencies":[]}"#,
            ),
            ("plugins/new.dll", "new"),
            ("config/custom.cfg", "defaults"),
        ] {
            zip.start_file(name, options).unwrap();
            zip.write_all(data.as_bytes()).unwrap();
        }
        let bytes = zip.finish().unwrap().into_inner();
        let p = package();
        assert!(
            !stage_package(&p, &p.versions[0], &bytes, root.path(), false)
                .unwrap()
                .enabled
        );
        assert!(!root
            .path()
            .join("BepInEx/plugins_disabled/Team-Mod/old.dll")
            .exists());
        assert!(root
            .path()
            .join("BepInEx/plugins_disabled/Team-Mod/new.dll")
            .exists());
        assert!(root
            .path()
            .join("BepInEx/plugins/Manual/manual.dll")
            .exists());
        assert_eq!(
            std::fs::read(root.path().join("BepInEx/config/custom.cfg")).unwrap(),
            b"user values"
        );
    }
    #[tokio::test]
    #[ignore = "downloads a public Hexium package to an isolated temporary directory; never runs it"]
    async fn live_hexium_package_stages_without_touching_game() {
        let packages =
            thunderstore_client::fetch_catalog(crate::models::profile::CatalogSource::Hexium, true)
                .await
                .unwrap();
        let p = thunderstore_client::find_package(&packages, "ValheimModding-Jotunn").unwrap();
        let v = &p.versions[0];
        let bytes = thunderstore_client::download_mod(&v.download_url)
            .await
            .unwrap();
        let root = tempfile::tempdir().unwrap();
        let installed = stage_package(p, v, &bytes, root.path(), false).unwrap();
        assert!(!installed.enabled);
        assert!(root
            .path()
            .join("BepInEx/plugins_disabled/ValheimModding-Jotunn")
            .is_dir());
        println!(
            "Staged {} {} in a temporary directory only",
            p.full_name, v.version_number
        );
    }
}
