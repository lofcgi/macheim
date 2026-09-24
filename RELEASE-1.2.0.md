# Macheim 1.2.0

## Reported issues

- Original thread: https://www.reddit.com/r/valheim/comments/1shj88g/
- Follow-up thread: https://www.reddit.com/r/valheim/comments/1w8s42o/
- New reports: p96iijy (refreshing indefinitely, M2/Valheim 1.0.12), p9dd5mz (external/custom Steam install not found, needs manual path).
- Original unanswered reports: p7rth2w (config blank), ox550q5 (console), p9794f5 (BepInEx response decode), paivatj (1.0.14), pb0hyrh (Hexium), ou71xve (Apple Silicon launch).
- Existing GitHub issues: #1 manual mods, #3 Gatekeeper, #7 version mismatch, #8 confirmation, #13 version selection, #17 downloads.
- Existing PRs #9/#10/#11 have equivalent fixes in 1.1.0; #12 updates requires safety review. Closed #18 is a CDN proposal, not merged.

## Findings

- Manual game path backend exists but is not exposed in UI, not persisted, and only validates existence.
- All catalog users fetch one large uncompressed JSON response with a 60-second deadline. Refresh does not actually force a refresh. Shared loading flags depend on mounted components.
- Mod downloads have no body idle deadline.
- BepInEx bootstrap unnecessarily depends on the entire catalog.
- PR #12 removes existing files before validating the replacement; do not merge unchanged.
- Native ARM modded play, Apple notarization, and universal shader repair cannot honestly be claimed by changing manager code alone. Keep explicit compatibility limits.

## Changes

- Compressed catalog indexes/chunks replace the monolithic request; bounded requests, persistent visible errors and explicit retry. Refresh now bypasses cache. Request-owned loading state survives navigation/unmount.
- Signed rating scores accepted; a real API response containing `-1` reproduced a decode failure. This is one verified failure mode, not proof of every historical user's network problem.
- BepInEx setup can query the loader directly without fetching the whole catalog.
- Validated, persisted custom game locations in setup and settings. Supports the app bundle, containing folder and Steam library. Missing external drives do not silently select another installation.
- New profiles can select Hexium. Existing profiles retain Thunderstore, with separate caches and no silent source migration. Missing catalog dependencies and unavailable pinned versions fail explicitly.
- Details expose all versions for explicit selection. Installed Mods offers opt-in update checks and individual confirmed updates. Version changes use a staged copy, validate the manifest, preserve configuration/disabled state, and retain recovery data if rollback fails. No automatic update or Update All is claimed.

## QA (2026-09-24)

- 14 frontend tests passed, including regressions first observed failing for catalog errors, refreshing after unmount, custom game path input and selecting an older installed-mod version.
- 25 offline Rust tests passed. Two live tests are explicitly excluded from normal CI and were run separately.
- Live Thunderstore index: 11,846 packages. Live Hexium index: 1,243 packages. Counts naturally change.
- Downloaded and ZIP-validated BepInExPack_Valheim 5.4.2351 (702,924 bytes) and Jotunn 2.30.2 (835,156 bytes), without installing into the user's game.
- Hexium Jotunn staged successfully into an isolated temporary directory in disabled state; downloaded code was not executed.
- Native packaged macOS app smoke test: startup/detection, populated catalog with Refreshing returning to Refresh, config file list and BepInEx.cfg contents, manual location controls, invalid location rejected with existing path unchanged, Hexium profile selector.
- Frontend production build, release metadata/lockfile alignment and bundled compatibility-plugin provenance verification passed.
- Final CI artifact verification and publication recorded in the PR/release, not assumed from local tests.

## Known limitations / issue disposition

- #1 manual-mod preservation, #7 version alignment and #8 confirmation have fixes already carried from 1.1.0 and regression coverage. #13 version selection is implemented here.
- PRs #9, #10 and #11 were already incorporated as equivalent commits in 1.1.0. PR #12 inspired the opt-in update workflow; it is not merged unchanged because its delete-before-install approach lacks the staged preservation used here.
- #3 remains a limitation: builds are ad-hoc signed, not Apple-notarized. No Developer ID identity is available on the build machine. Rosetta is still used for modded play; native ARM support is not claimed.
- #17 antivirus-specific CDN blocking was not reproduced on this machine. Current public package downloads pass; timeouts/errors are improved, but no security bypass or unverified mirror rewrite is shipped. Keep this issue open for affected-environment evidence.
- Hexium and Thunderstore remain separate per profile. No cross-repository migration, mixed-source dependency resolution or PC/r2modman profile-code import is claimed.
- The visual compatibility plugin is unchanged: four version-pinned item effects for Valheim 0.221.12/Unity 6000.0.61f1/ShaderHelperForMac 3.3.0. Not a fix for every pink material, UI icon, backpack or 1.0.x mod. These manager tests are not a Valheim 1.0.14 multiplayer/gameplay certification.
- Updating shared patchers follows existing package routing; Macheim does not infer ownership of unrelated/shared files and does not delete them speculatively. Keep independent world and mod backups.

## Research sources (checked 2026-09-24)

Official service/source evidence:
- https://thunderstore.io/c/valheim/api/v1/package-listing-index/
- https://thunderstore.io/api/experimental/package/denikson/BepInExPack_Valheim/
- https://valheim.hexium.gg/api/v1/package-listing-index/
- https://hexium.gg/faq
- https://github.com/Kesomannen/gale/blob/master/src-tauri/src/thunderstore/backend.rs (endpoint discovery; code not copied)
- https://github.com/BepInEx/BepInEx/issues/899 (upstream ARM discussion, not a compatibility guarantee)

Community reports are the two Reddit threads linked above and GitHub issues/PRs, not evidence of universal compatibility.
