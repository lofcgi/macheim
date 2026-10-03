use serde::{Deserialize, Serialize};

/// Thunderstore mod manifest (manifest.json inside ZIP)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub name: String,
    pub version_number: String,
    pub description: String,
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub website_url: String,
    #[serde(default)]
    pub author: Option<String>,
}

impl Manifest {
    /// Thunderstore accepts manifests saved with a UTF-8 BOM, which serde_json rejects.
    pub fn from_slice(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes))
    }
}
