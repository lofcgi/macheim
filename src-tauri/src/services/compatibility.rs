//! Version-pinned, opt-out runtime compatibility. Never edits upstream mod assets.
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{AppError, AppResult};
use crate::models::Profile;

pub const MANAGED_DIR: &str = "Macheim-ItemMaterialCompat";
pub const DLL_NAME: &str = "Macheim.ItemMaterialCompat.dll";
const CONFIG: &str = "com.macheim.itemmaterialcompat.cfg";
const PLUGIN: &[u8] =
    include_bytes!("../../resources/compatibility/Macheim.ItemMaterialCompat.dll");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatibilitySettings {
    pub automatic: bool,
    #[serde(default)]
    pub disabled_rules: Vec<String>,
}

impl Default for CompatibilitySettings {
    fn default() -> Self {
        Self {
            automatic: true,
            disabled_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub package: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    pub package: String,
    pub version: String,
    pub title: String,
    pub prefabs: Vec<String>,
    pub reason: String,
    pub validation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub revision: u32,
    pub plugin_version: String,
    pub game_version: String,
    pub unity_version: String,
    pub requirements: Vec<Requirement>,
    pub rules: Vec<Rule>,
}

pub fn catalog() -> Catalog {
    serde_json::from_str(include_str!("../../../compatibility/catalog.json"))
        .expect("valid bundled catalog")
}

#[derive(Debug, Serialize)]
pub struct RuleStatus {
    pub rule: Rule,
    pub eligible: bool,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct CompatibilityStatus {
    pub profile_name: String,
    pub settings: CompatibilitySettings,
    pub catalog: Catalog,
    pub rules: Vec<RuleStatus>,
    pub installed: bool,
    pub up_to_date: bool,
    pub game_running: bool,
    pub recent_log: Vec<String>,
}

fn package_ready(profile: &Profile, root: &Path, package: &str, version: &str) -> bool {
    profile
        .mods
        .iter()
        .any(|m| m.full_name == package && m.version == version && m.enabled)
        && contains_dll(&root.join("BepInEx/plugins").join(package))
}

fn contains_dll(path: &Path) -> bool {
    if path.is_symlink() {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|e| {
        let path = e.path();
        if path.is_symlink() {
            return false;
        }
        if path.is_dir() {
            contains_dll(&path)
        } else {
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("dll"))
        }
    })
}

pub fn rule_statuses(profile: &Profile, root: &Path) -> Vec<RuleStatus> {
    let catalog = catalog();
    catalog.rules.iter().map(|rule| {
        let reason = if !profile.compatibility.automatic {
            "Automatic compatibility is off for this profile.".into()
        } else if profile.compatibility.disabled_rules.contains(&rule.id) {
            "This rule is disabled for this profile.".into()
        } else if !package_ready(profile, root, &rule.package, &rule.version) {
            format!("Requires enabled {} {} with installed plugin files. Other versions are not verified.", rule.package, rule.version)
        } else if let Some(req) = catalog.requirements.iter().find(|req| !package_ready(profile, root, &req.package, &req.version)) {
            format!("Requires enabled {} {}. Macheim does not install it silently.", req.package, req.version)
        } else { String::new() };
        RuleStatus { rule: rule.clone(), eligible: reason.is_empty(), reason }
    }).collect()
}

fn desired_config(rules: &[RuleStatus]) -> String {
    let prefabs: Vec<_> = rules
        .iter()
        .filter(|r| r.eligible)
        .flat_map(|r| r.rule.prefabs.clone())
        .collect();
    format!("## Managed by Macheim's Mac Compatibility page. Changes require a game restart.\n[General]\nEnabled = {}\n\n[Scope]\nItem Prefabs = {}\nItem Prefab Prefixes =\nItem Prefab Suffixes =\n\n[Rendering]\nParticle Shader = Sprites/Default\n", !prefabs.is_empty(), prefabs.join(","))
}

pub fn status(profile: &Profile, root: &Path) -> AppResult<CompatibilityStatus> {
    let rules = rule_statuses(profile, root);
    let expected = rules.iter().any(|r| r.eligible);
    let plugin = root
        .join("BepInEx/plugins")
        .join(MANAGED_DIR)
        .join(DLL_NAME);
    let installed = plugin.is_file();
    let up_to_date = if expected {
        std::fs::read(&plugin).is_ok_and(|b| b == PLUGIN)
            && std::fs::read_to_string(root.join("BepInEx/config").join(CONFIG))
                .is_ok_and(|c| c == desired_config(&rules))
    } else {
        !installed
    };
    Ok(CompatibilityStatus {
        profile_name: profile.name.clone(),
        settings: profile.compatibility.clone(),
        catalog: catalog(),
        rules,
        installed,
        up_to_date,
        game_running: super::launcher::is_game_running()?,
        recent_log: read_recent_log(root)?,
    })
}

/// Stage and atomically replace one file; used for metadata as well as the managed plugin.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> AppResult<()> {
    let parent = path
        .parent()
        .ok_or_else(|| AppError::Mod("Missing parent directory".into()))?;
    reject_symlink_ancestors(path)?;
    std::fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| AppError::Io(e.error))?;
    Ok(())
}

pub fn reject_symlink_ancestors(path: &Path) -> AppResult<()> {
    for ancestor in path.ancestors() {
        // Standard macOS aliases to /private, not user-controlled mod symlinks.
        if ["/var", "/tmp", "/etc"]
            .iter()
            .any(|p| ancestor == Path::new(p))
        {
            continue;
        }
        if ancestor.is_symlink() {
            return Err(AppError::Mod(format!(
                "Refusing to replace a symlinked path: {}",
                ancestor.display()
            )));
        }
    }
    Ok(())
}

/// Only our named plugin/config are touched. A receipt protects customized binaries.
pub fn reconcile(profile: &Profile, root: &Path) -> AppResult<()> {
    super::launcher::ensure_game_stopped()?;
    reconcile_files(profile, root)
}

fn reconcile_files(profile: &Profile, root: &Path) -> AppResult<()> {
    let bepinex = root.join("BepInEx");
    if !bepinex.join("core/BepInEx.dll").is_file() {
        return Ok(());
    }
    let rules = rule_statuses(profile, root);
    let enabled = rules.iter().any(|r| r.eligible);
    let plugin = bepinex.join("plugins").join(MANAGED_DIR).join(DLL_NAME);
    let config = bepinex.join("config").join(CONFIG);
    let receipt = bepinex.join(".macheim-compatibility.json");
    let known_hash = std::fs::read_to_string(&receipt)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["sha256"].as_str().map(str::to_owned));
    if plugin.exists() {
        reject_symlink_ancestors(&plugin)?;
        let bytes = std::fs::read(&plugin)?;
        if bytes != PLUGIN && known_hash.as_deref() != Some(&hex::encode(Sha256::digest(&bytes))) {
            return Err(AppError::Mod("The managed compatibility DLL was customized. Back it up and remove it manually before applying.".into()));
        }
    }
    if enabled {
        // Avoid loading our GUID twice alongside a manually installed local test build.
        for entry in std::fs::read_dir(bepinex.join("plugins"))? {
            let entry = entry?;
            if entry.file_name() != MANAGED_DIR && contains_named_dll(&entry.path()) {
                return Err(AppError::Mod(format!("Another {} exists in {}. Disable that manual copy before using managed compatibility.", DLL_NAME, entry.path().display())));
            }
        }
        let desired = desired_config(&rules);
        if std::fs::read_to_string(&config).is_ok_and(|c| c != desired) {
            let backup = bepinex.join(".macheim-compat-backups").join(format!(
                "{}-{}",
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
                CONFIG
            ));
            atomic_write(&backup, &std::fs::read(&config)?)?;
        }
        // Config first: a crash cannot activate a broader stale scope with the new DLL.
        atomic_write(&config, desired.as_bytes())?;
        atomic_write(&plugin, PLUGIN)?;
        atomic_write(
            &receipt,
            serde_json::to_string_pretty(&serde_json::json!({
                "plugin_version": catalog().plugin_version,
                "sha256": hex::encode(Sha256::digest(PLUGIN)),
                "profile": profile.name
            }))?
            .as_bytes(),
        )?;
    } else if plugin.exists() {
        // Keep a recoverable copy outside BepInEx's plugin discovery directories.
        let backup = bepinex.join(".macheim-compat-backups").join(format!(
            "{}-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
            DLL_NAME
        ));
        atomic_write(&backup, &std::fs::read(&plugin)?)?;
        std::fs::remove_file(&plugin)?;
        // Leave unrelated files/configuration untouched; the DLL no longer loads.
    }
    Ok(())
}

fn contains_named_dll(path: &Path) -> bool {
    if path.is_symlink() {
        return true;
    } // cannot safely prove no duplicate through a link
    if path.is_file() {
        return path.file_name().is_some_and(|n| n == DLL_NAME);
    }
    std::fs::read_dir(path).is_ok_and(|entries| {
        entries
            .filter_map(Result::ok)
            .any(|e| contains_named_dll(&e.path()))
    })
}

fn read_recent_log(root: &Path) -> AppResult<Vec<String>> {
    let path = root.join("BepInEx/LogOutput.log");
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let mut file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    file.seek(SeekFrom::Start(size.saturating_sub(256 * 1024)))?;
    let mut bytes = Vec::new();
    file.take(256 * 1024).read_to_end(&mut bytes)?;
    let mut lines: Vec<_> = String::from_utf8_lossy(&bytes)
        .lines()
        .filter(|line| line.contains("Macheim Item Material Compatibility"))
        .map(|line| line.chars().take(800).collect::<String>())
        .collect();
    if lines.len() > 30 {
        lines.drain(..lines.len() - 30);
    }
    Ok(lines)
}

pub fn validate_settings(settings: &CompatibilitySettings) -> AppResult<()> {
    let catalog = catalog();
    let known: HashSet<_> = catalog.rules.iter().map(|r| &r.id).collect();
    if settings.disabled_rules.iter().any(|id| !known.contains(id)) {
        return Err(AppError::Mod("Unknown compatibility rule".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::InstalledMod;

    fn fixture() -> (tempfile::TempDir, Profile) {
        let root = tempfile::tempdir().unwrap();
        let mut profile = Profile::new("Test".into(), "".into());
        for (package, version) in [
            ("Therzie-Wizardry", "1.1.8"),
            ("DrummerCraig-ShaderHelperForMac", "3.3.0"),
        ] {
            let dir = root.path().join("BepInEx/plugins").join(package);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("Original.dll"), b"untouched").unwrap();
            profile.mods.push(InstalledMod {
                source: None,
                full_name: package.into(),
                name: package.into(),
                author: "Test".into(),
                version: version.into(),
                enabled: true,
                description: "".into(),
                dependencies: vec![],
                installed_at: "".into(),
                icon: "".into(),
            });
        }
        std::fs::create_dir_all(root.path().join("BepInEx/core")).unwrap();
        std::fs::write(root.path().join("BepInEx/core/BepInEx.dll"), b"loader").unwrap();
        (root, profile)
    }

    #[test]
    fn exact_version_and_enabled_dependencies_are_required() {
        let (root, mut profile) = fixture();
        assert_eq!(
            rule_statuses(&profile, root.path())
                .iter()
                .filter(|r| r.eligible)
                .count(),
            1
        );
        profile.mods[0].version = "1.1.9".into();
        assert!(rule_statuses(&profile, root.path())
            .iter()
            .all(|r| !r.eligible));
        profile.mods[0].version = "1.1.8".into();
        profile.mods[1].enabled = false;
        assert!(rule_statuses(&profile, root.path())
            .iter()
            .all(|r| !r.eligible));
    }

    #[test]
    fn apply_disable_and_reenable_preserve_originals() {
        let (root, mut profile) = fixture();
        reconcile_files(&profile, root.path()).unwrap();
        let plugin = root
            .path()
            .join("BepInEx/plugins")
            .join(MANAGED_DIR)
            .join(DLL_NAME);
        assert_eq!(std::fs::read(&plugin).unwrap(), PLUGIN);
        let config =
            std::fs::read_to_string(root.path().join("BepInEx/config").join(CONFIG)).unwrap();
        assert!(config.contains("Item Prefabs = ArcaneScroll_BlackForest_TW,ShardBonemass_TW"));
        assert!(!config.contains("kg_EnchantScroll"));
        profile.compatibility.automatic = false;
        reconcile_files(&profile, root.path()).unwrap();
        assert!(!plugin.exists());
        assert_eq!(
            std::fs::read(
                root.path()
                    .join("BepInEx/plugins/Therzie-Wizardry/Original.dll")
            )
            .unwrap(),
            b"untouched"
        );
        profile.compatibility.automatic = true;
        reconcile_files(&profile, root.path()).unwrap();
        assert!(plugin.exists());
    }

    #[test]
    fn unsupported_update_removes_old_automatic_patch() {
        let (root, mut profile) = fixture();
        reconcile_files(&profile, root.path()).unwrap();
        profile.mods[0].version = "2.0.0".into();
        reconcile_files(&profile, root.path()).unwrap();
        assert!(!root
            .path()
            .join("BepInEx/plugins")
            .join(MANAGED_DIR)
            .join(DLL_NAME)
            .exists());
    }

    #[test]
    fn customized_binary_and_manual_duplicates_are_not_overwritten() {
        let (root, profile) = fixture();
        let dir = root.path().join("BepInEx/plugins/Manual");
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join(DLL_NAME), b"manual").unwrap();
        assert!(reconcile_files(&profile, root.path()).is_err());
        assert_eq!(std::fs::read(dir.join(DLL_NAME)).unwrap(), b"manual");
    }

    #[test]
    fn legacy_profile_gets_safe_catalog_default() {
        let p: Profile = serde_json::from_str(
            r#"{"name":"Old","description":"","mods":[],"created_at":"","updated_at":""}"#,
        )
        .unwrap();
        assert!(p.compatibility.automatic);
        let root = tempfile::tempdir().unwrap();
        assert!(rule_statuses(&p, root.path()).iter().all(|r| !r.eligible));
    }

    #[test]
    fn per_rule_opt_out_and_unknown_rule_validation() {
        let (root, mut p) = fixture();
        p.compatibility
            .disabled_rules
            .push("wizardry-drop-particles".into());
        assert!(rule_statuses(&p, root.path()).iter().all(|r| !r.eligible));
        assert!(validate_settings(&p.compatibility).is_ok());
        p.compatibility.disabled_rules.push("unknown".into());
        assert!(validate_settings(&p.compatibility).is_err());
    }
}
