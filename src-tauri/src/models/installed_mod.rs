use serde::{Deserialize, Serialize};

/// Represents an installed mod in a profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledMod {
    #[serde(default)]
    pub source: Option<super::profile::CatalogSource>,
    /// Thunderstore full name: "Author-ModName"
    pub full_name: String,
    pub author: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub enabled: bool,
    pub dependencies: Vec<String>,
    pub installed_at: String,
    /// Icon URL from Thunderstore
    #[serde(default)]
    pub icon: String,
}

impl InstalledMod {
    /// Directory name used for storing mod files: "Author-ModName"
    pub fn dir_name(&self) -> String {
        self.full_name.clone()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn installed_source_survives_round_trip() {
        let data = serde_json::json!({"full_name":"Team-Mod","author":"Team","name":"Mod","version":"1.0.0","description":"","enabled":true,"dependencies":[],"installed_at":"","source":"hexium"});
        let installed: super::InstalledMod = serde_json::from_value(data).unwrap();
        assert_eq!(serde_json::to_value(installed).unwrap()["source"], "hexium");
    }
}
