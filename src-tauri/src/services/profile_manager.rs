use super::{
    compatibility::{atomic_write, reject_symlink_ancestors, MANAGED_DIR},
    thunderstore_client,
};
use crate::error::{AppError, AppResult};
use crate::models::{InstalledMod, Profile};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::warn;

const SYNC_DIRS: [&str; 4] = ["plugins", "patchers", "config", "plugins_disabled"];
const ACTIVE_MARKER: &str = ".macheim-active-profile";

pub fn validate_name(name: &str) -> AppResult<()> {
    if name.is_empty()
        || name.len() > 120
        || name.starts_with('.')
        || name.trim() != name
        || name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
    {
        return Err(AppError::Profile("Use a nonempty name without path separators, a leading dot or surrounding spaces (max 120 bytes).".into()));
    }
    Ok(())
}
pub fn get_profiles_dir() -> PathBuf {
    thunderstore_client::get_app_data_dir().join("profiles")
}
pub fn get_profile_dir(name: &str) -> PathBuf {
    get_profiles_dir().join(name)
}
pub fn ensure_default_profile() -> AppResult<()> {
    if !get_profile_dir("Default").exists() {
        create_profile("Default", "Default mod profile")?;
    }
    Ok(())
}
pub fn create_profile(name: &str, description: &str) -> AppResult<Profile> {
    validate_name(name)?;
    let dir = get_profile_dir(name);
    reject_symlink_ancestors(&dir)?;
    std::fs::create_dir_all(get_profiles_dir())?;
    std::fs::create_dir(&dir)?;
    for sub in SYNC_DIRS {
        std::fs::create_dir_all(dir.join("BepInEx").join(sub))?;
    }
    let profile = Profile::new(name.into(), description.into());
    save_profile(&profile)?;
    Ok(profile)
}
pub fn load_profile(name: &str) -> AppResult<Profile> {
    validate_name(name)?;
    let path = get_profile_dir(name).join("profile.json");
    reject_symlink_ancestors(&path)?;
    let profile: Profile = serde_json::from_slice(&std::fs::read(path)?)?;
    if profile.name != name {
        return Err(AppError::Profile(
            "Profile metadata name does not match its directory.".into(),
        ));
    }
    for m in &profile.mods {
        validate_name(&m.full_name)?;
    }
    Ok(profile)
}
pub fn save_profile(profile: &Profile) -> AppResult<()> {
    validate_name(&profile.name)?;
    for m in &profile.mods {
        validate_name(&m.full_name)?;
    }
    atomic_write(
        &get_profile_dir(&profile.name).join("profile.json"),
        &serde_json::to_vec_pretty(profile)?,
    )
}
pub fn list_profiles() -> AppResult<Vec<Profile>> {
    ensure_default_profile()?;
    let mut profiles = Vec::new();
    for entry in std::fs::read_dir(get_profiles_dir())? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            match load_profile(&name) {
                Ok(p) => profiles.push(p),
                Err(e) => warn!("Cannot load profile {}: {}", name, e),
            }
        }
    }
    profiles.sort_by_key(|p| (p.name != "Default", p.name.clone()));
    Ok(profiles)
}
pub fn delete_profile(name: &str) -> AppResult<()> {
    validate_name(name)?;
    if name == "Default" {
        return Err(AppError::Profile(
            "Cannot delete the default profile".into(),
        ));
    }
    load_profile(name)?;
    let archive = thunderstore_client::get_app_data_dir().join("deleted-profiles");
    std::fs::create_dir_all(&archive)?;
    std::fs::rename(
        get_profile_dir(name),
        archive.join(format!(
            "{}-{}",
            name,
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        )),
    )?;
    Ok(())
}
pub fn clone_profile(source_name: &str, new_name: &str) -> AppResult<Profile> {
    let mut profile = load_profile(source_name)?;
    validate_name(new_name)?;
    let target = get_profile_dir(new_name);
    if target.exists() {
        return Err(AppError::Profile("Profile already exists".into()));
    }
    copy_dir_recursive(&get_profile_dir(source_name), &target)?;
    profile.name = new_name.into();
    profile.description = format!("Cloned from {}", source_name);
    profile.touch();
    save_profile(&profile)?;
    Ok(profile)
}
pub fn set_active_profile(name: &str, root: &Path) -> AppResult<()> {
    validate_name(name)?;
    atomic_write(&root.join(ACTIVE_MARKER), name.as_bytes())
}
/// Legacy releases had no active marker. Preserve ambiguous live files in a new
/// recovery profile instead of importing another profile's contents into Default.
pub fn initialize_game_profile(root: &Path) -> AppResult<String> {
    if let Ok(name) = std::fs::read_to_string(root.join(ACTIVE_MARKER)) {
        let name = name.trim();
        if load_profile(name).is_ok() {
            import_existing_mods(name, root)?;
            return Ok(name.into());
        }
        return Err(AppError::Profile("The saved active profile is missing or invalid. Restore that profile before continuing; game files were not changed.".into()));
    }
    let profiles = list_profiles()?;
    let has_live_data = SYNC_DIRS.iter().any(|s| {
        std::fs::read_dir(root.join("BepInEx").join(s)).is_ok_and(|mut e| e.next().is_some())
    });
    let name = if (has_live_data && profiles.len() > 1) || profiles.iter().any(|p| !p.mods.is_empty()) {
        let name = format!(
            "Recovered-{}",
            chrono::Utc::now().format("%Y%m%d-%H%M%S-%f")
        );
        create_profile(&name, "Preserved live installation from an older release with no saved active profile. Existing profiles were not overwritten.")?;
        name
    } else {
        "Default".into()
    };
    import_existing_mods(&name, root)?;
    set_active_profile(&name, root)?;
    Ok(name)
}
pub fn switch_profile(name: &str, root: &Path) -> AppResult<()> {
    load_profile(name)?;
    replace_bepinex_dirs(
        &get_profile_dir(name).join("BepInEx"),
        &root.join("BepInEx"),
    )
}
pub fn save_game_state_to_profile(name: &str, root: &Path) -> AppResult<()> {
    let mut profile = load_profile(name)?;
    let source = root.join("BepInEx");
    if !source.exists() {
        return Ok(());
    }
    register_manual_mods(&mut profile, &source)?;
    replace_bepinex_dirs(&source, &get_profile_dir(name).join("BepInEx"))?;
    profile.touch();
    save_profile(&profile)
}
pub fn import_existing_mods(name: &str, root: &Path) -> AppResult<Vec<String>> {
    let mut profile = load_profile(name)?;
    let added = register_manual_mods(&mut profile, &root.join("BepInEx"))?;
    if root.join("BepInEx").exists() {
        replace_bepinex_dirs(
            &root.join("BepInEx"),
            &get_profile_dir(name).join("BepInEx"),
        )?;
    }
    profile.touch();
    save_profile(&profile)?;
    Ok(added)
}
fn register_manual_mods(profile: &mut Profile, bepinex: &Path) -> AppResult<Vec<String>> {
    let mut tracked: HashSet<_> = profile.mods.iter().map(|m| m.full_name.clone()).collect();
    profile.mods.retain(|m| m.full_name != MANAGED_DIR);
    let mut added = Vec::new();
    for (dir, enabled) in [("plugins", true), ("plugins_disabled", false)] {
        let path = bepinex.join(dir);
        if !path.exists() {
            continue;
        }
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let ty = entry.file_type()?;
            if name.starts_with('.') || name == MANAGED_DIR || tracked.contains(&name) {
                continue;
            }
            if !(ty.is_dir()
                || (ty.is_file()
                    && entry
                        .path()
                        .extension()
                        .is_some_and(|e| e.eq_ignore_ascii_case("dll"))))
            {
                continue;
            }
            validate_name(&name)?;
            let (author, mod_name) = name.split_once('-').unwrap_or(("Unknown", &name));
            profile.mods.push(InstalledMod {
                full_name: name.clone(),
                author: author.into(),
                name: mod_name.into(),
                version: "0.0.0".into(),
                description: "Manually installed (version unverified)".into(),
                enabled,
                dependencies: vec![],
                installed_at: chrono::Utc::now().to_rfc3339(),
                icon: String::new(),
            });
            tracked.insert(name.clone());
            added.push(name);
        }
    }
    Ok(added)
}
pub fn add_mod_to_profile(name: &str, installed: InstalledMod) -> AppResult<()> {
    let mut p = load_profile(name)?;
    p.mods.retain(|m| m.full_name != installed.full_name);
    p.mods.push(installed);
    p.touch();
    save_profile(&p)
}
pub fn remove_mod_from_profile(name: &str, full_name: &str) -> AppResult<()> {
    let mut p = load_profile(name)?;
    p.mods.retain(|m| m.full_name != full_name);
    p.touch();
    save_profile(&p)
}
pub fn update_mod_enabled(name: &str, full_name: &str, enabled: bool) -> AppResult<()> {
    let mut p = load_profile(name)?;
    if let Some(m) = p.mods.iter_mut().find(|m| m.full_name == full_name) {
        m.enabled = enabled;
    }
    p.touch();
    save_profile(&p)
}
pub fn export_profile(name: &str) -> AppResult<String> {
    Ok(serde_json::to_string_pretty(&load_profile(name)?)?)
}
pub fn import_profile(json: &str, new_name: Option<&str>) -> AppResult<Profile> {
    let mut p: Profile = serde_json::from_str(json)?;
    if let Some(name) = new_name {
        p.name = name.into();
    }
    validate_name(&p.name)?;
    for m in &p.mods {
        validate_name(&m.full_name)?;
    }
    create_profile(&p.name, &p.description)?;
    p.touch();
    save_profile(&p)?;
    Ok(p)
}
/// Stage every directory before replacing any data. Restore outgoing directories
/// on rename failure; retain recovery files if rollback itself cannot complete.
pub(crate) fn replace_bepinex_dirs(source: &Path, target: &Path) -> AppResult<()> {
    reject_symlink_ancestors(source)?;
    reject_symlink_ancestors(target)?;
    std::fs::create_dir_all(target)?;
    let stage = tempfile::Builder::new()
        .prefix(".macheim-stage-")
        .tempdir_in(target)?;
    for sub in SYNC_DIRS {
        reject_symlink_ancestors(&target.join(sub))?;
        let incoming = stage.path().join(format!("new-{}", sub));
        if source.join(sub).exists() {
            copy_dir_recursive(&source.join(sub), &incoming)?;
        } else {
            std::fs::create_dir(&incoming)?;
        }
    }
    let mut moved = Vec::new();
    let result: AppResult<()> = (|| {
        for sub in SYNC_DIRS {
            let dest = target.join(sub);
            let old = stage.path().join(format!("old-{}", sub));
            if dest.exists() {
                std::fs::rename(&dest, &old)?;
            }
            moved.push(sub);
            std::fs::rename(stage.path().join(format!("new-{}", sub)), &dest)?;
        }
        Ok(())
    })();
    if let Err(error) = result {
        let mut rollback_failed = false;
        for sub in moved.into_iter().rev() {
            let dest = target.join(sub);
            if dest.exists()
                && std::fs::rename(&dest, stage.path().join(format!("failed-{}", sub))).is_err()
            {
                rollback_failed = true;
                continue;
            }
            let old = stage.path().join(format!("old-{}", sub));
            if old.exists() && std::fs::rename(old, dest).is_err() {
                rollback_failed = true;
            }
        }
        if rollback_failed {
            let recovery = stage.keep();
            return Err(AppError::Profile(format!(
                "{}; recovery files retained at {}",
                error,
                recovery.display()
            )));
        }
        return Err(error);
    }
    Ok(())
}
fn copy_dir_recursive(source: &Path, target: &Path) -> AppResult<()> {
    reject_symlink_ancestors(source)?;
    reject_symlink_ancestors(target)?;
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let src = entry.path();
        if entry.file_type()?.is_symlink() {
            return Err(AppError::Profile(format!(
                "Back up or resolve this symlink before switching profiles: {}",
                src.display()
            )));
        }
        let dst = target.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else {
            std::fs::copy(src, dst)?;
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_paths_and_empty_profile_names() {
        for name in [
            "",
            "..",
            "../Default",
            "/tmp/data",
            "a/b",
            "a\\b",
            ".hidden",
            " Default ",
        ] {
            assert!(validate_name(name).is_err());
        }
        assert!(validate_name("RelicHeim-Mac-Test").is_ok());
    }
    #[test]
    fn round_trip_preserves_loose_disabled_and_config_files() {
        let game = tempfile::tempdir().unwrap();
        let saved = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(game.path().join("plugins/Manual")).unwrap();
        std::fs::write(game.path().join("plugins/Manual/mod.dll"), b"manual").unwrap();
        std::fs::write(game.path().join("plugins/Loose.dll"), b"loose").unwrap();
        std::fs::create_dir_all(game.path().join("plugins_disabled/Disabled")).unwrap();
        std::fs::write(
            game.path().join("plugins_disabled/Disabled/mod.dll"),
            b"disabled",
        )
        .unwrap();
        std::fs::create_dir_all(game.path().join("config")).unwrap();
        std::fs::write(game.path().join("config/custom.cfg"), b"custom").unwrap();
        let mut p = Profile::new("Test".into(), "".into());
        assert_eq!(register_manual_mods(&mut p, game.path()).unwrap().len(), 3);
        assert!(
            !p.mods
                .iter()
                .find(|m| m.full_name == "Disabled")
                .unwrap()
                .enabled
        );
        replace_bepinex_dirs(game.path(), saved.path()).unwrap();
        let restored = tempfile::tempdir().unwrap();
        replace_bepinex_dirs(saved.path(), restored.path()).unwrap();
        assert_eq!(
            std::fs::read(restored.path().join("plugins/Loose.dll")).unwrap(),
            b"loose"
        );
        assert_eq!(
            std::fs::read(restored.path().join("config/custom.cfg")).unwrap(),
            b"custom"
        );
    }
    #[test]
    fn missing_disabled_folder_does_not_resurrect_mods() {
        let game = tempfile::tempdir().unwrap();
        let saved = tempfile::tempdir().unwrap();
        std::fs::create_dir(saved.path().join("plugins_disabled")).unwrap();
        std::fs::write(saved.path().join("plugins_disabled/Old.dll"), b"old").unwrap();
        replace_bepinex_dirs(game.path(), saved.path()).unwrap();
        assert!(!saved.path().join("plugins_disabled/Old.dll").exists());
    }
    #[cfg(unix)]
    #[test]
    fn failed_staging_leaves_live_data_intact() {
        let source = tempfile::tempdir().unwrap();
        let live = tempfile::tempdir().unwrap();
        std::fs::create_dir(source.path().join("plugins")).unwrap();
        std::os::unix::fs::symlink("/missing", source.path().join("plugins/Unsafe")).unwrap();
        std::fs::create_dir(live.path().join("plugins")).unwrap();
        std::fs::write(live.path().join("plugins/Original.dll"), b"preserve").unwrap();
        assert!(replace_bepinex_dirs(source.path(), live.path()).is_err());
        assert_eq!(
            std::fs::read(live.path().join("plugins/Original.dll")).unwrap(),
            b"preserve"
        );
    }
}
