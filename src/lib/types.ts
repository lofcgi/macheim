export interface ThunderstorePackage {
  name: string;
  full_name: string;
  owner: string;
  description: string;
  version_number: string;
  rating_score: number;
  downloads: number;
  is_deprecated: boolean;
  icon: string;
  categories: string[];
  date_updated: string;
}

export interface PackageVersion {
  name: string;
  full_name: string;
  version_number: string;
  dependencies: string[];
  download_url: string;
  downloads: number;
  description: string;
  icon: string;
  date_created: string;
}

export interface PackageDetail {
  name: string;
  full_name: string;
  owner: string;
  package_url: string;
  date_updated: string;
  is_deprecated: boolean;
  rating_score: number;
  versions: PackageVersion[];
  categories: string[];
}

export interface InstalledMod {
  full_name: string;
  name: string;
  author: string;
  version: string;
  enabled: boolean;
  description: string;
  icon: string;
  dependencies: string[];
  installed_at: string;
}

export interface Profile {
  name: string;
  description: string;
  catalog_source?: "thunderstore" | "hexium";
  mods: InstalledMod[];
  compatibility: CompatibilitySettings;
  created_at: string;
  updated_at: string;
}

export interface GameStatus {
  installed: boolean;
  game_path: string | null;
  bepinex_installed: boolean;
  active_profile: string;
}

export interface ConfigFileSummary {
  path: string;
  filename: string;
  size: number;
}

export interface ConfigFile {
  path: string;
  filename: string;
  sections: ConfigSection[];
}

export interface ConfigSection {
  name: string;
  entries: ConfigEntry[];
}

export interface ConfigEntry {
  key: string;
  value: string;
  setting_type: string | null;
  default_value: string | null;
  description: string | null;
  acceptable_values: string | null;
  acceptable_value_range: string | null;
}

export interface BackupInfo {
  filename: string;
  profile_name: string;
  created_at: string;
  size: number;
  path: string;
}

export interface CompatibilitySettings { automatic: boolean; disabled_rules: string[]; }
export interface CompatibilityRule {
  id: string; package: string; version: string; title: string;
  prefabs: string[]; reason: string; validation: string;
}
export interface CompatibilityStatus {
  profile_name: string;
  settings: CompatibilitySettings;
  catalog: {
    revision: number; plugin_version: string; game_version: string; unity_version: string;
    requirements: { package: string; version: string }[]; rules: CompatibilityRule[];
  };
  rules: { rule: CompatibilityRule; eligible: boolean; reason: string }[];
  installed: boolean; up_to_date: boolean; game_running: boolean; recent_log: string[];
}

export type Page =
  | "setup"
  | "browse"
  | "installed"
  | "modpacks"
  | "config"
  | "profiles"
  | "compatibility"
  | "settings";

export interface Toast {
  id: string;
  type: "success" | "error" | "info" | "warning";
  message: string;
  duration?: number;
}

export type SortOption = "downloads" | "rating" | "updated" | "name";
export type SortDirection = "asc" | "desc";
