# Macheim 1.3.1 — manifest BOM hotfix

## Changes

- Thunderstore archives whose `manifest.json` starts with a UTF-8 byte order mark now install and update. 1.3.0 rejected them with `JSON error: expected value at line 1 column 1` (#25) because every install started validating the archive manifest, and serde_json does not accept a BOM.
- Affected 1.3.0 packages included ASharpPen-Drop_That 3.1.6 and Thunderstore ValheimModding-Jotunn 2.30.2, so mods depending on Thunderstore Jotunn failed when it was not yet installed. Hexium packages and mixed-source profiles were not the cause.

## QA — 2026-10-03

- Reproduced the reported error with a failing regression test (`manifest_with_utf8_bom_is_accepted`) before the fix.
- The real ASharpPen-Drop_That 3.1.6 archive (manifest begins `EF BB BF`) staged successfully after the fix.
- Manifest scan of the top 1,000 Thunderstore packages by downloads plus all ASharpPen packages: 203 of 1,003 used a UTF-8 BOM and are fixed by this release. Hexium top 400: none affected.
- Live staging into isolated temporary profiles: Hexium Jotunn + Thunderstore Drop_That in one mixed profile, and Thunderstore Jotunn + Spawn_That. No mod code executed and no user game files changed.
- Rust: 42 offline tests and 4 opt-in live tests passed. Frontend: 20 tests passed; TypeScript and production build passed.
- Bundled shader policy: 26 checks passed; plugin unchanged. Release version/provenance verification passed for 1.3.1.

## Limits

- Manifests in UTF-16 are still rejected with an explicit error. The scan found one (`Nextek-SpeedyPaths` 1.0.9).
- All 1.3.0 limits still apply: both catalogs must load, mixed sources do not guarantee runtime compatibility, no new shader coverage, Apple notarization or native ARM modded-game support.

## Sources

- Report: https://github.com/lofcgi/macheim/issues/25
- Package: https://thunderstore.io/c/valheim/p/ASharpPen/Drop_That/ (checked 2026-10-03)
