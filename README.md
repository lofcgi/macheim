# Macheim

### Valheim Mod Manager for macOS

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS-blue.svg)]()
[![Latest Release](https://img.shields.io/github/v/release/lofcgi/macheim)](https://github.com/lofcgi/macheim/releases/latest)
[![Downloads](https://img.shields.io/github/downloads/lofcgi/macheim/total.svg)](https://github.com/lofcgi/macheim/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%20v2-orange.svg)](https://tauri.app)

> [r2modman](https://github.com/ebkr/r2modmanPlus) and Thunderstore Mod Manager don't support macOS.
> **Macheim** fills that gap.

A native macOS mod manager for [Valheim](https://store.steampowered.com/app/892970/Valheim/), built with Tauri v2. Browse, install, and manage mods from [Thunderstore](https://thunderstore.io/c/valheim/) with a single click. No terminal required.

## Screenshots

| Mod Browser | Installed Mods | Modpacks |
|:---:|:---:|:---:|
| ![Browse Mods](screenshots/browse-mods.png) | ![Installed Mods](screenshots/installed-mods.png) | ![Modpacks](screenshots/modpacks.png) |

## Features

- **Auto-detect Valheim** - Automatically finds your Valheim installation via Steam library
- **One-click BepInEx install** - Downloads and configures BepInEx from Thunderstore, handles macOS Gatekeeper automatically
- **Thunderstore mod browser** - Browse, search, and filter thousands of mods (Popular / Newest / Top Rated / A-Z)
- **One-click mod install** - Automatic dependency resolution using topological sort
- **Modpack support** - Install entire modpacks with all dependencies in one click
- **Profile management** - Create and switch profiles; remember the active profile across restarts and preserve manual mods
- **Mac Compatibility** - Automatically apply version-pinned visual workarounds for tested item effects, with per-profile and per-rule opt-out
- **BepInEx config editor** - Edit mod configuration files directly in the app
- **Backup & restore** - Back up profile metadata and configs (not mod binaries or worlds); restore into a separate profile
- **Sync & Clean** - Re-download missing enabled mod files; confirm before moving unmanaged folders to recoverable storage
- **Play Modded** - Launch Valheim with mods, automatically handles Rosetta for Apple Silicon
- **Dark viking-themed UI** - Built for the Valheim aesthetic
- **Lightweight** - 5.7MB DMG, 16MB app (vs Electron-based alternatives at ~1.3GB)

## Requirements

- **macOS 12+** (Monterey or later)
- **Apple Silicon** (M1/M2/M3/M4) or **Intel** Mac
- **Valheim** installed via Steam
- Internet connection for downloading mods

## Installation

### Download

1. Download `Macheim.dmg` from the [Releases](https://github.com/lofcgi/macheim/releases) page
2. Open the DMG and drag **Macheim** to your Applications folder
3. **Important:** The app is ad-hoc signed, but not Apple-notarized, so macOS may block it. First try **System Settings → Privacy & Security → Open Anyway**. If macOS instead reports that the app is damaged, open Terminal and run:
   ```bash
   xattr -cr /Applications/Macheim.app
   ```
4. Now open Macheim normally. Only remove the quarantine attribute after confirming that you downloaded Macheim from this repository's Releases page.

### Build from Source

Prerequisites: [Node.js](https://nodejs.org/) 24 LTS, current stable [Rust/Cargo](https://rustup.rs/), and Xcode Command Line Tools. The Tauri CLI is a project dependency. If the build says `cargo metadata` cannot be found, install Rust with rustup and restart your terminal.

```bash
git clone https://github.com/lofcgi/macheim.git
cd macheim
npm ci
npm test
npm run verify:release
npm run tauri build
```

The built DMG will be in `src-tauri/target/release/bundle/dmg/`.

The application embeds only Macheim's own compatibility DLL, alongside its source
and a SHA-256/source manifest. Building the manager does not require Valheim or
.NET. To rebuild the plugin itself, install .NET 9 and Valheim/BepInEx locally,
then run `sh scripts/build-compatibility.sh`. No game or third-party reference DLLs
are redistributed. See [compatibility details](tools/item-material-compat/README.md).

## Catalogs, updates and custom game locations (1.2.0)

- Setup and Settings accept the Valheim app path or its containing folder, including external Steam libraries. The selection is checked and saved. If that drive is disconnected, reconnect it or select another installation; Macheim does not silently switch installations.
- New profiles can use **Thunderstore** or **Hexium**. Existing profiles remain on Thunderstore. Catalogs are kept separate to avoid silently changing the source of installed mods; automatic migration and mixed-source resolution are not provided. Missing dependencies produce an error instead of an invented replacement.
- **Installed Mods → Check updates** fetches current catalog data. Updates are opt-in, one mod at a time; match your server/modpack's requirements first. A dependency that is too old or disabled must be addressed first.
- A mod's details now include **Version to install**, including older releases. Version changes stage the profile before replacing live files and preserve existing configuration and disabled state. Keep independent backups of saves and manually installed mods.
- Catalog errors stay visible with a retry button. BepInEx setup uses a dedicated package lookup instead of waiting for the entire catalog.

See [the 1.2.0 release audit](RELEASE-1.2.0.md) for tested behavior and remaining limitations.

## Mac Compatibility (1.1.0)

Open **Mac Compatibility → Check support** to inspect the bundled catalog and the
active profile. This is a mod/version check, not a visual scan of every shader.
Eligible rules are reconciled after mod changes and before **Play Modded**.
Quit Valheim before applying or disabling them.

The initial catalog covers four tested dropped items: VES F weapon and blessed F
weapon scrolls (Valheim Enchantment System 1.9.12), and the Wizardry 1.1.8 Black
Forest scroll and Bonemass shard. ShaderHelperForMac **3.3.0** must already be
enabled; Macheim does not silently install or reset it. Runtime checks restrict the
patch to Valheim **0.221.12**, Unity **6000.0.61f1**, and macOS Metal.

- Disable **Automatically apply verified compatibility rules** to unload Macheim's
  patch on the next launch, or disable an individual rule. Other shader mods remain active.
- Updates outside the verified mod versions are skipped; a previously managed patch
  is removed when no rules remain eligible.
- The plugin clones runtime materials/textures; upstream mod assets are unchanged.
- No universal repair, all-mod scanner, creature/building/UI fixes, or Windows visual parity is promised.
- Logs are shown locally, never uploaded. A missing log entry is not a compatibility pass.

See [the 1.1.0 release audit](RELEASE-1.1.0.md) for issue/PR coverage and remaining limitations.

### Profile and recovery notes

When upgrading an older installation with multiple profiles and no active-profile
record, the live mod files are preserved in a new `Recovered-…` profile. Existing
profiles are not overwritten. Switch to your preferred profile after reviewing it.
Symlinked mod/profile paths are refused during replacement; back them up and resolve
the links first. Do not change profiles while Valheim is running.

Saved profiles: `~/Library/Application Support/com.macheim/profiles`.
Removed profiles: `~/Library/Application Support/com.macheim/deleted-profiles`.
Cleaned mod folders: `<Valheim>/BepInEx/.macheim-clean-backups`.
Managed compatibility backups: `<Valheim>/BepInEx/.macheim-compat-backups`.
These are local recovery copies; keep your own backup of worlds and manual mods.

## Getting Started

1. **Launch Macheim** - The Setup Wizard will automatically detect your Valheim installation
2. **Install BepInEx** - Click "Install BepInEx" to set up the mod framework
3. **Browse Mods** - Go to the Mods tab to browse and search Thunderstore
4. **Install** - Click any mod to see details, then click "Install" to download with all dependencies
5. **Play Modded** - Click "Play Modded" to launch Valheim with your mods enabled

## Known Issues

### Pink/Magenta Objects

Some mod-added objects (buildings, creatures, effects) may render as pink/magenta or with incorrect transparency. Causes include missing Metal shader variants and incompatible material/shader settings. A shader reported as supported can still render incorrectly.

**The 1.1.0 workarounds cover only the objects listed above.** Visual errors can make items or effects hard to see; other shader helpers can also affect shared materials or UI. Report the mod version, affected object and a screenshot. Do not assume that every pink object has the same cause.

### BepInEx Requires Rosetta

Macheim's supported modded launch path uses `arch -x86_64` and requires Rosetta on Apple Silicon. The manager app itself has a native Apple Silicon build; that does **not** mean the modded game runs natively on ARM. Experimental native-ARM BepInEx builds are not integrated or supported here.

### Multiplayer and other mod managers

Macheim does not translate mods or synchronize a server's mod requirements. Use the
required compatible mod versions on every client/server. This release's visual patch
does not change item stats or network behavior, but cross-platform multiplayer was
not an end-to-end validation of this release. Future Valheim 1.0 compatibility is not
guaranteed. r2modman/Thunderstore profile-code import is not supported; Macheim's
metadata import/export backend is a different format and is not a PC profile importer.

### macOS Gatekeeper

After BepInEx installation, macOS may block some libraries. Macheim automatically removes quarantine attributes, but if you encounter issues, go to **System Settings > Privacy & Security** to allow blocked items.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | [Tauri v2](https://tauri.app/) |
| Backend | Rust (28 source files, 25 Tauri commands) |
| Frontend | React 19 + TypeScript + Tailwind CSS v4 |
| State | Zustand |
| UI Icons | Lucide React |

## How It Works

Macheim uses Tauri v2 to bridge a Rust backend with a React frontend:

- **Game detection**: Parses Steam's `libraryfolders.vdf` to locate Valheim
- **BepInEx management**: Downloads from Thunderstore, patches config for macOS (`Type = GameObject`), removes Gatekeeper quarantine from dylibs
- **Mod installation**: Downloads mod ZIPs, extracts to the correct profile directory, resolves dependencies via Kahn's algorithm (topological sort)
- **Game launch**: Uses `arch -x86_64 env DYLD_INSERT_LIBRARIES=libdoorstop.dylib` to load BepInEx into the game under Rosetta on Apple Silicon; SIP is not disabled

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [Tauri](https://tauri.app/) - Lightweight app framework
- [BepInEx](https://github.com/BepInEx/BepInEx) - Unity mod loader framework
- [Thunderstore](https://thunderstore.io/) - Mod repository and API
- [r2modmanPlus](https://github.com/ebkr/r2modmanPlus) - Inspiration for this project
- The Valheim modding community
