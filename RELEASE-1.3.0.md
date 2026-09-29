# Macheim 1.3.0 — mixed-source profiles

## Changes

- Browse Mods and Modpacks expose All sources / Thunderstore / Hexium filters. Source labels distinguish duplicate package identities; selection passes the explicit origin to details and installs.
- Every managed install records its origin. Updates and sync stay on that source even when another repository publishes a newer version. Explicit cross-source replacement is rejected; back up and uninstall before changing origin.
- Legacy managed mods inherit their previous profile catalog when loaded; no existing files move. Manually imported unknown-version mods remain unknown-source and cannot be silently overwritten by catalog updates.
- New dependencies prefer the selected root's source, retain existing dependency origins, and may use dependencies unique to the other source. Batch-update dependencies available on both sources require explicit installation first. Version conflicts stop the transaction.
- Card installs and modpacks use the existing validated staging/rollback path instead of partial incremental installation. Configs, disabled state and unrelated files are retained. BepInEx remains the shared setup-managed loader; its dedicated lookup is independent of browsing state.

## QA — 2026-09-28

- Reproduced missing installed-origin persistence with a failing regression test before implementation.
- Rust offline tests: 41 passed (four live checks are opt-in).
- Frontend: 20 tests passed; TypeScript and production build passed.
- Bundled shader policy: 26 checks passed; plugin unchanged.
- Release version/provenance verification passed for 1.3.0; local ad-hoc-signed ARM app build passed.
- Live catalog/loader check: 12,144 Thunderstore and 1,355 Hexium entries decoded. BepInExPack 5.4.2351 and Jotunn archives downloaded and ZIP-validated.
- Live mixed staging: Hexium ValheimModding-Jotunn and Thunderstore Alpus-Transmog installed sequentially into one isolated temporary profile with recorded origins and serialized metadata. No mod code executed and no user game files changed by these tests.
- Native ARM UI smoke: 1.3.0 setup label, unified catalog, Hexium filter, Jotunn Hexium details/source URL, and 49 existing installed mods retaining Thunderstore origin verified. No live mod installation/update, profile switch or gameplay performed in the user's installation.
- Synthetic tests cover source pinning, missing-origin refusal, ambiguous duplicates, legacy/manual metadata, mixed-source dependencies, one shared loader, conflicting versions, failed downloads/manifests and preservation of config/disabled/unrelated files.

## Limits

- Both catalogs must load successfully; failure is explicit with retry, never a silent fallback. Source changes are not automatic migration and PC profile-code import is not implemented.
- Mixed sources do not guarantee runtime compatibility or matching server versions. Current live Alpus-Transmog download success does not establish the cause of a user's report: request exact package URL, selected source, application/game/macOS versions, action and error screenshot.
- No new shader coverage, Apple notarization or native ARM modded-game support. Existing shader version guards remain unchanged. Intel runtime and full gameplay have not been tested locally.
- Public ARM/Intel release artifact checks are recorded after the release build, separately from local QA.
