# Macheim 1.2.1

Audit and verification date: 2026-09-25 (KST).

## Changes

- Dependency planning now validates selected versions and the final enabled profile before mutation. Detects cycles, shared version conflicts, old/disabled transitive dependencies, malformed dependency strings and reverse-dependency breakage. Existing dependencies are not silently upgraded or enabled; errors explain how to select compatible versions.
- Update All is opt-in, confirms the listed versions and profile, and uses one operation lock and one staging transaction. All archives and manifests (including dependency lists) are validated before live directory replacement. Download/ZIP failures preserve live files; metadata-save failure invokes filesystem rollback. Configs, manual files and disabled state are retained. Recovery data is retained when rollback itself fails.
- Settings → Mod downloads lets users choose the Thunderstore server default, Cloudflare, Google or Hetzner CDN. Only recognized HTTPS package CDN URLs are rewritten, including redirect destinations before contacting them. Hexium URLs are unchanged. This is an explicit preference, not an automatic fallback or security-software exemption.
- Compatibility prominently shows the last recorded runtime status and clearly warns that it may belong to a prior session/profile. Missing logs mean unverified, not successful. Backpacks are explicitly outside the bundled item patch scope.

## QA

- Rust: 38/38 passed, including three explicit live tests (Thunderstore/Hexium catalog and loader decoding, Hexium package staging in a temp directory, default versus explicit Hetzner CDN SHA-256 equality).
- Frontend: 18/18 passed. TypeScript/Vite production build and release metadata/plugin provenance verification passed.
- Shader policy: 26 checks passed; shader plugin unchanged.
- Native Apple Silicon `.app` release build succeeded. Native UI smoke check confirmed version 1.2.1, readable existing installed mods, CDN settings and unverified runtime status without applying changes to the user's mods or launching the game.
- Regression coverage includes conflict/cycle failures reproduced against 1.2.0, transitive/reverse constraints, batch ordering, blocked/invalid second download preserving live files, manifest dependency mismatch, CDN allowlisting, batch confirmation, saved CDN preferences and runtime rejection visibility.
- Live Hexium testing exposed catalog-added loader dependencies absent from the upstream ZIP. Validation now allows catalog additions but rejects archive requirements not covered by the validated catalog plan. A regression test covers this case.

## Remaining limits

- The manager is ad-hoc signed, not Apple-notarized. Developer ID signing/notarization and independent fresh-download Gatekeeper QA remain necessary; development certificates do not substitute for distribution signing.
- Modded Valheim still launches via `arch -x86_64`/Rosetta. A native ARM manager does not imply native ARM mod loading.
- The bundled visual patch still requires Valheim 0.221.12 / Unity 6000.0.61f1 / ShaderHelperForMac 3.3.0 and exact catalog mod versions. It covers four tested item prefabs, not backpacks, arbitrary monsters/buildings, or every shader. No new game version or full-gameplay compatibility claim is made.
- A working alternate CDN download does not establish that Malwarebytes permits it on every affected machine. Issue #17 remains open pending affected-user validation. Do not disable security software.
- Plugin dependency versions are treated as minimum requirements for already-installed or explicitly selected versions; uninstalled dependencies default to the catalog's stated version. Conflicts abort rather than silently changing the user's selection.
- BepInEx loader updates remain in setup, outside regular plugin updates. Manual mods without dependency metadata cannot have undeclared requirements verified.
- Staging protects against handled download/install failures; it is not a claim of crash/power-loss atomicity across multiple directories. Regular fresh installs and Sync & Clean are not converted into batch-update transactions by this release.

## Sources

- Apple distribution/notarization: https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution (checked 2026-09-25).
- r2modman CDN definitions: https://github.com/ebkr/r2modmanPlus/blob/master/src/providers/cdn/CdnHostList.ts (checked 2026-09-25).
- Reports: https://github.com/lofcgi/macheim/issues/3 and https://github.com/lofcgi/macheim/issues/17.
- Batch-update proposal: https://github.com/lofcgi/macheim/pull/12. This release uses staging rather than its delete-before-install implementation.
