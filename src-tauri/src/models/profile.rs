use serde::{Deserialize, Serialize};

use super::installed_mod::InstalledMod;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CatalogSource {
    #[default]
    Thunderstore,
    Hexium,
}

/// A mod profile containing a set of mods and their configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub mods: Vec<InstalledMod>,
    #[serde(default)]
    pub catalog_source: CatalogSource,
    #[serde(default)]
    pub compatibility: crate::services::compatibility::CompatibilitySettings,
    pub created_at: String,
    pub updated_at: String,
}

impl Profile {
    pub fn new(name: String, description: String) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        Self {
            name,
            description,
            mods: Vec::new(),
            catalog_source: CatalogSource::default(),
            compatibility: Default::default(),
            created_at: now.clone(),
            updated_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_profiles_keep_thunderstore_and_hexium_round_trips() {
        let mut data = serde_json::to_value(Profile::new("Test".into(), "".into())).unwrap();
        data.as_object_mut().unwrap().remove("catalog_source");
        assert_eq!(
            serde_json::from_value::<Profile>(data.clone())
                .unwrap()
                .catalog_source,
            CatalogSource::Thunderstore
        );
        data["catalog_source"] = serde_json::json!("hexium");
        assert_eq!(
            serde_json::from_value::<Profile>(data.clone())
                .unwrap()
                .catalog_source,
            CatalogSource::Hexium
        );
        data["catalog_source"] = serde_json::json!("untrusted");
        assert!(serde_json::from_value::<Profile>(data).is_err());
    }
}
