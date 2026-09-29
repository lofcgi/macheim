use crate::{error::{AppError, AppResult}, models::{InstalledMod, ThunderstorePackage, profile::CatalogSource}};

/// An installed identity keeps its origin even when another catalog has a newer copy.
/// For new dependencies, prefer the explicitly selected root's catalog; unique
/// dependencies from the other catalog are allowed. Batch ambiguity is an error
/// in the dependency planner, not an arbitrary first match.
pub fn select_catalog(
    packages: &[ThunderstorePackage],
    installed: &[InstalledMod],
    preferred: Option<CatalogSource>,
) -> Vec<ThunderstorePackage> {
    let installed: std::collections::HashMap<_, _> = installed.iter().map(|m| (m.full_name.as_str(), m)).collect();
    let preferred_names: std::collections::HashSet<_> = packages.iter().filter(|p| Some(p.source) == preferred).map(|p| p.full_name.as_str()).collect();
    packages.iter().filter(|p| {
        if let Some(m) = installed.get(p.full_name.as_str()) {
            return m.source == Some(p.source);
        }
        match preferred {
            Some(source) if preferred_names.contains(p.full_name.as_str()) => p.source == source,
            _ => true,
        }
    }).cloned().collect()
}

pub fn validate_target_source(installed: &[InstalledMod], name: &str, requested: Option<CatalogSource>) -> AppResult<()> {
    if let Some(m) = installed.iter().find(|m| m.full_name == name) {
        if m.source.is_none() {
            return Err(AppError::Mod(format!("{name} has an unknown/manual source. Back it up and remove it explicitly before installing a catalog version.")));
        }
        if requested.is_some() && requested != m.source {
            return Err(AppError::Mod(format!("{name} is already installed from {:?}. Switching sources is not automatic; keep that source or back up and uninstall it first.", m.source.unwrap())));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn package(source: CatalogSource) -> ThunderstorePackage {
        serde_json::from_value(serde_json::json!({"source":source,"name":"Mod","owner":"Team","full_name":"Team-Mod","package_url":"","date_updated":"","is_deprecated":false,"rating_score":0,"versions":[]})).unwrap()
    }
    fn installed(source: Option<CatalogSource>) -> InstalledMod {
        serde_json::from_value(serde_json::json!({"source":source,"name":"Mod","author":"Team","full_name":"Team-Mod","version":"1.0.0","description":"","enabled":true,"dependencies":[],"installed_at":""})).unwrap()
    }
    #[test]
    fn pins_updates_and_never_substitutes_missing_origin() {
        use CatalogSource::*;
        let entries = vec![package(Thunderstore), package(Hexium)];
        let mods = vec![installed(Some(Hexium))];
        assert_eq!(select_catalog(&entries, &mods, Some(Thunderstore))[0].source, Hexium);
        assert!(select_catalog(&entries[..1], &mods, None).is_empty());
        assert!(validate_target_source(&mods, "Team-Mod", Some(Thunderstore)).is_err());
        assert!(validate_target_source(&[installed(None)], "Team-Mod", Some(Hexium)).is_err());
    }
    #[test]
    fn explicit_root_preference_deduplicates_without_hiding_unique_dependencies() {
        use CatalogSource::*;
        let mut unique = package(Hexium); unique.full_name = "Team-Unique".into();
        let entries = vec![package(Thunderstore), package(Hexium), unique];
        let selected = select_catalog(&entries, &[], Some(Thunderstore));
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].source, Thunderstore);
        assert_eq!(selected[1].source, Hexium);
        assert_eq!(select_catalog(&entries, &[], None).len(), 3);
    }
    #[test]
    fn legacy_profiles_pin_to_old_catalog_without_relabeling_manual_mods() {
        let mut profile = crate::models::Profile::new("Test".into(), "".into());
        profile.catalog_source = CatalogSource::Hexium;
        profile.mods = vec![installed(None), installed(Some(CatalogSource::Thunderstore))];
        let mut manual = installed(None); manual.version = "0.0.0".into();
        profile.mods.push(manual);
        profile.pin_legacy_sources();
        assert_eq!(profile.mods[0].source, Some(CatalogSource::Hexium));
        assert_eq!(profile.mods[1].source, Some(CatalogSource::Thunderstore));
        assert_eq!(profile.mods[2].source, None);
    }
}
