use crate::error::{AppError, AppResult};
use crate::models::thunderstore::ParsedDependency;
use crate::models::{InstalledMod, ThunderstorePackage};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResolvedDependency {
    pub full_name: String,
    pub author: String,
    pub name: String,
    pub version: String,
    pub download_url: String,
    pub description: String,
    pub icon: String,
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::DependencyResolution(message.into())
}

/// Validate the final profile, not just the first level of the selected mods.
/// Existing dependencies are never silently upgraded or enabled.
pub fn resolve_plan(
    targets: &[(String, String)],
    packages: &[ThunderstorePackage],
    installed: &[InstalledMod],
) -> AppResult<Vec<ResolvedDependency>> {
    struct Planner<'a> {
        packages: &'a [ThunderstorePackage],
        installed: &'a [InstalledMod],
        selected: HashMap<String, String>,
        choices: HashMap<String, String>,
        visiting: HashSet<String>,
        done: HashSet<String>,
        result: Vec<ResolvedDependency>,
    }
    impl Planner<'_> {
        fn visit(&mut self, id: &str, required: Option<&str>) -> AppResult<()> {
            // BepInEx is managed by setup, outside regular plugin transactions.
            if id == "denikson-BepInExPack_Valheim" {
                return Ok(());
            }
            let existing = self.installed.iter().find(|m| m.full_name == id);
            if required.is_some() && existing.is_some_and(|m| !m.enabled) {
                return Err(invalid(format!(
                    "Enable dependency {id} first. No files were changed."
                )));
            }
            let version = self
                .selected
                .get(id)
                .cloned()
                .or_else(|| existing.map(|m| m.version.clone()))
                .or_else(|| self.choices.get(id).cloned())
                .or_else(|| required.map(str::to_owned))
                .ok_or_else(|| invalid(format!("No version selected for {id}")))?;
            if let Some(required) = required {
                let actual = semver::Version::parse(&version).map_err(|_| {
                    invalid(format!(
                        "Cannot verify installed version of {id}: {version}"
                    ))
                })?;
                let minimum = semver::Version::parse(required)
                    .map_err(|_| invalid(format!("Invalid dependency version {id}-{required}")))?;
                if actual < minimum {
                    return Err(invalid(format!("Dependency conflict: {id} needs at least {required}, but {version} is selected/installed. Update it explicitly first or include it in the batch. No files were changed.")));
                }
            }
            if self.visiting.contains(id) {
                return Err(invalid(format!("Circular dependency involving {id}")));
            }
            if self.done.contains(id) {
                return Ok(());
            }
            if self.visiting.len() >= 128 {
                return Err(invalid("Dependency graph exceeds the safe depth limit"));
            }
            self.choices.insert(id.into(), version.clone());
            self.visiting.insert(id.into());
            let download = self.selected.contains_key(id) || existing.is_none();
            let (dependencies, resolved) = if download {
                let p = self
                    .packages
                    .iter()
                    .find(|p| p.full_name == id)
                    .ok_or_else(|| {
                        invalid(format!(
                            "Package {id} is missing from this profile's catalog"
                        ))
                    })?;
                let v = p
                    .versions
                    .iter()
                    .find(|v| v.version_number == version)
                    .ok_or_else(|| {
                        invalid(format!("Required version {version} of {id} is unavailable"))
                    })?;
                (
                    v.dependencies.clone(),
                    Some(ResolvedDependency {
                        full_name: id.into(),
                        author: p.owner.clone(),
                        name: p.name.clone(),
                        version: version.clone(),
                        download_url: v.download_url.clone(),
                        description: v.description.clone(),
                        icon: v.icon.clone(),
                    }),
                )
            } else {
                (existing.unwrap().dependencies.clone(), None)
            };
            for dependency in dependencies {
                let parsed = ParsedDependency::parse(&dependency)
                    .ok_or_else(|| invalid(format!("Malformed dependency {dependency} in {id}")))?;
                self.visit(&parsed.full_name, Some(&parsed.version))?;
            }
            self.visiting.remove(id);
            self.done.insert(id.into());
            if let Some(resolved) = resolved {
                self.result.push(resolved);
            }
            Ok(())
        }
    }
    let mut selected = HashMap::new();
    for (id, version) in targets {
        if selected.insert(id.clone(), version.clone()).is_some() {
            return Err(invalid(format!("Duplicate update target {id}")));
        }
    }
    let mut planner = Planner {
        packages,
        installed,
        selected,
        choices: HashMap::new(),
        visiting: HashSet::new(),
        done: HashSet::new(),
        result: vec![],
    };
    for (id, _) in targets {
        planner.visit(id, None)?;
    }
    for m in installed.iter().filter(|m| m.enabled) {
        planner.visit(&m.full_name, None)?;
    }
    Ok(planner.result)
}

pub fn resolve_dependencies(
    target: &str,
    version: &str,
    packages: &[ThunderstorePackage],
    installed: &[InstalledMod],
) -> AppResult<Vec<ResolvedDependency>> {
    Ok(
        resolve_plan(&[(target.into(), version.into())], packages, installed)?
            .into_iter()
            .filter(|p| p.full_name != target)
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn package(name: &str, deps: Vec<&str>) -> ThunderstorePackage {
        serde_json::from_value(serde_json::json!({
            "name":name,"full_name":format!("Team-{name}"),"owner":"Team","package_url":"",
            "date_updated":"","is_deprecated":false,"rating_score":0,
            "versions":[{"name":name,"full_name":format!("Team-{name}-1.0.0"),"version_number":"1.0.0",
            "dependencies":deps,"download_url":"","downloads":0,"description":"","icon":"","date_created":""}]
        })).unwrap()
    }
    fn installed(name: &str, version: &str, enabled: bool, deps: Vec<&str>) -> InstalledMod {
        InstalledMod {
            full_name: format!("Team-{name}"),
            name: name.into(),
            author: "Team".into(),
            version: version.into(),
            enabled,
            dependencies: deps.into_iter().map(str::to_owned).collect(),
            description: String::new(),
            icon: String::new(),
            installed_at: String::new(),
        }
    }
    #[test]
    fn missing_dependencies_are_errors_not_successful_partial_plans() {
        assert!(resolve_dependencies(
            "Team-A",
            "1.0.0",
            &[package("A", vec!["Missing-Mod-1.0.0"])],
            &[]
        )
        .is_err());
    }
    #[test]
    fn unavailable_pinned_version_is_never_replaced_with_latest() {
        assert!(resolve_dependencies("Team-A", "0.9.0", &[package("A", vec![])], &[]).is_err());
    }
    #[test]
    fn rejects_shared_version_conflicts() {
        let mut d = package("D", vec![]);
        let mut v = d.versions[0].clone();
        v.version_number = "2.0.0".into();
        d.versions.push(v);
        let packages = vec![
            package("A", vec!["Team-B-1.0.0", "Team-C-1.0.0"]),
            package("B", vec!["Team-D-1.0.0"]),
            package("C", vec!["Team-D-2.0.0"]),
            d,
        ];
        assert!(resolve_dependencies("Team-A", "1.0.0", &packages, &[]).is_err());
    }
    #[test]
    fn rejects_cycles() {
        let packages = vec![
            package("A", vec!["Team-B-1.0.0"]),
            package("B", vec!["Team-C-1.0.0"]),
            package("C", vec!["Team-B-1.0.0"]),
        ];
        assert!(resolve_dependencies("Team-A", "1.0.0", &packages, &[]).is_err());
    }
    #[test]
    fn rejects_old_or_disabled_transitive_dependency() {
        let packages = vec![
            package("A", vec!["Team-B-1.0.0"]),
            package("B", vec!["Team-C-2.0.0"]),
        ];
        for c in [
            installed("C", "1.0.0", true, vec![]),
            installed("C", "2.0.0", false, vec![]),
        ] {
            assert!(resolve_dependencies("Team-A", "1.0.0", &packages, &[c]).is_err());
        }
    }
    #[test]
    fn rejects_reverse_dependency_breakage() {
        assert!(resolve_plan(
            &[("Team-D".into(), "1.0.0".into())],
            &[package("D", vec![])],
            &[
                installed("A", "1.0.0", true, vec!["Team-D-2.0.0"]),
                installed("D", "2.0.0", true, vec![])
            ]
        )
        .is_err());
    }
    #[test]
    fn batch_validates_final_versions_and_orders_dependencies_first() {
        let packages = vec![package("A", vec!["Team-B-1.0.0"]), package("B", vec![])];
        let result = resolve_plan(
            &[
                ("Team-A".into(), "1.0.0".into()),
                ("Team-B".into(), "1.0.0".into()),
            ],
            &packages,
            &[installed("B", "0.9.0", true, vec![])],
        )
        .unwrap();
        assert_eq!(
            result
                .iter()
                .map(|p| p.full_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Team-B", "Team-A"]
        );
    }
    #[test]
    fn malformed_dependencies_are_errors() {
        assert!(
            resolve_dependencies("Team-A", "1.0.0", &[package("A", vec!["invalid"])], &[]).is_err()
        );
    }
}
