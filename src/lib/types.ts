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
  mod_count: number;
  created_at: string;
  last_used: string;
}

export interface GameStatus {
  installed: boolean;
  game_path: string | null;
  bepinex_installed: boolean;
  active_profile: string;
}

export interface ConfigFile {
  filename: string;
  mod_name: string;
  sections: ConfigSection[];
}

export interface ConfigSection {
  name: string;
  entries: ConfigEntry[];
}

export interface ConfigEntry {
  key: string;
  value: string;
  setting_type: string;
  default_value: string;
  description: string;
  acceptable_values: string[] | null;
  acceptable_range: [string, string] | null;
}

export interface BackupInfo {
  filename: string;
  profile_name: string;
  created_at: string;
  mod_count: number;
}

export type Page =
  | "setup"
  | "browse"
  | "installed"
  | "modpacks"
  | "config"
  | "profiles"
  | "settings";

export interface Toast {
  id: string;
  type: "success" | "error" | "info" | "warning";
  message: string;
  duration?: number;
}

export type SortOption = "downloads" | "rating" | "updated" | "name";
export type SortDirection = "asc" | "desc";
