# Macheim 1.4.0 — in-app updates

## Changes

- **In-app updates** (#33). Macheim checks `https://github.com/lofcgi/macheim/releases/latest/download/latest.json` when it starts, using the Tauri updater. If a newer version exists, a bar at the top of the window (on the setup screen and in the main window) shows the new and installed versions, a **Release notes** link and **Install and restart**. Its close button hides the bar for the session.
- **Settings → App updates** shows the installed version, can check again, and shows the last check or install error. A failed automatic check (for example, offline) is shown there instead of a pop-up.
- Updates are `.app.tar.gz` archives signed with Macheim's own minisign key. The public key is built into the app; the private key is only in the release workflow's secrets. A download whose signature does not match is refused and the installed app stays as it was.
- Release builds add `src-tauri/tauri.release.conf.json` (`createUpdaterArtifacts`), so `npm run tauri build` from source still works without the signing key.
- Tauri stays at 2.10. The updater (2.12.0) and process (2.3.1) plugins are the newest versions that support it, and `@tauri-apps/api` is pinned to 2.10 so the CLI's crate/npm version check passes.

## QA — 2026-10-19

Machine: macOS 26.2, Apple Silicon. The test builds used a throwaway signing key and a local update feed (`http://127.0.0.1`, `dangerousInsecureTransportProtocol`); release builds use the production key and the GitHub URL.

- A 1.4.0 build was installed in `/Applications`, ad-hoc signed and owned by the user (the same layout as a drag-installed DMG). At launch it requested `latest.json`, and the bar showed `Macheim 1.4.1 is available. You have 1.4.0.` on the setup screen. Accessibility frames showed the bar in its own row above the setup content.
- Feed signed for different bytes: after **Install and restart**, the archive was downloaded and the bar showed `Update failed: The signature verification failed. You can also download it from the release notes page.` The app stayed 1.4.0 and kept running.
- Valid feed: **Install and restart** downloaded the archive and replaced `/Applications/Macheim.app` with 1.4.1. Macheim restarted as a new process without a password prompt or Gatekeeper warning, and the new process checked the feed again. `codesign --verify --deep --strict` passed on the replaced app, and it had no `com.apple.quarantine` attribute. After restart there was no bar; **Settings → App updates** showed `v1.4.1`, and **Check for updates** reported `Macheim is up to date.`
- `npm run tauri build` without `TAURI_SIGNING_PRIVATE_KEY` finished (source builds). With the release config and a key it produced `Macheim.app.tar.gz` and `.sig`.
- Frontend: 5 new tests (up to date, progress then relaunch, failed install shown and retryable without relaunch, failed check shown in Settings, dismiss) and 21 existing tests pass; TypeScript and production build pass. Rust: 47 offline tests pass (4 opt-in live tests not run). Release verification now also checks the updater endpoint, the public key, the release config and the workflow flag.

## Limits

- 1.3.3 and older have no updater, so users must download 1.4.0 by hand once.
- The Intel build was not update-tested on an Intel Mac. CI writes both `darwin-aarch64` and `darwin-x86_64` into `latest.json`.
- If Macheim runs from a location the user cannot write to (for example straight from the DMG), the update fails and the error is shown. If macOS denies the replacement outright, the updater asks for an administrator password. That path was not triggered in testing.
- The updater signature proves the archive came from Macheim's release workflow. It is not Apple notarization: a new download from the Releases page still needs the steps in the installation notes.
- All 1.3.3 limits still apply.

## Sources

- Request: https://github.com/lofcgi/macheim/issues/33
- Tauri updater plugin: https://v2.tauri.app/plugin/updater/ (checked 2026-10-19)
