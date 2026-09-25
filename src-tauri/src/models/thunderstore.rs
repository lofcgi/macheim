use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThunderstorePackage {
    pub name: String,
    pub full_name: String,
    pub owner: String,
    pub package_url: String,
    pub date_updated: String,
    pub is_deprecated: bool,
    pub rating_score: i64,
    pub versions: Vec<PackageVersion>,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub is_pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub name: String,
    pub full_name: String,
    pub version_number: String,
    pub dependencies: Vec<String>,
    pub download_url: String,
    pub downloads: u64,
    pub description: String,
    pub icon: String,
    pub date_created: String,
    #[serde(default)]
    pub file_size: u64,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub uuid4: Option<String>,
}

/// Lightweight package info for search results / listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageListing {
    pub name: String,
    pub full_name: String,
    pub owner: String,
    pub description: String,
    pub version_number: String,
    pub rating_score: i64,
    pub downloads: u64,
    pub is_deprecated: bool,
    pub icon: String,
    pub categories: Vec<String>,
    pub date_updated: String,
}

impl From<&ThunderstorePackage> for PackageListing {
    fn from(pkg: &ThunderstorePackage) -> Self {
        let latest = pkg.versions.first();
        Self {
            name: pkg.name.clone(),
            full_name: pkg.full_name.clone(),
            owner: pkg.owner.clone(),
            description: latest.map(|v| v.description.clone()).unwrap_or_default(),
            version_number: latest.map(|v| v.version_number.clone()).unwrap_or_default(),
            rating_score: pkg.rating_score,
            downloads: latest.map(|v| v.downloads).unwrap_or(0),
            is_deprecated: pkg.is_deprecated,
            icon: latest.map(|v| v.icon.clone()).unwrap_or_default(),
            categories: pkg.categories.clone(),
            date_updated: pkg.date_updated.clone(),
        }
    }
}

/// Parsed dependency string: "Author-ModName-1.2.3"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedDependency {
    pub author: String,
    pub name: String,
    pub full_name: String,
    pub version: String,
}

impl ParsedDependency {
    /// Parse a Thunderstore dependency string like "Author-ModName-1.2.3"
    pub fn parse(dep: &str) -> Option<Self> {
        // Format: "Author-Name-Major.Minor.Patch"
        // Split from the right to find the version part
        let parts: Vec<&str> = dep.split('-').collect();
        if parts.len() < 3 {
            return None;
        }

        // The version is always the last part and contains dots
        // Author is first, name is everything between
        let author = parts[0].to_string();

        // Find where version starts (last part that looks like a semver)
        // Walk backwards to find the version segment
        let mut version_idx = parts.len();
        for i in (1..parts.len()).rev() {
            if parts[i].contains('.') {
                version_idx = i;
                break;
            }
        }

        if version_idx >= parts.len() || version_idx < 2 {
            // Fallback: assume last part is version
            if parts.len() == 3 {
                return Some(Self {
                    author: parts[0].to_string(),
                    name: parts[1].to_string(),
                    full_name: format!("{}-{}", parts[0], parts[1]),
                    version: parts[2].to_string(),
                });
            }
            return None;
        }

        let name = parts[1..version_idx].join("-");
        let version = parts[version_idx..].join("-");
        let full_name = format!("{}-{}", author, name);

        Some(Self {
            author,
            name,
            full_name,
            version,
        })
    }
}
