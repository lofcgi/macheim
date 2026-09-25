import { useState } from "react";
import {
  FolderOpen,
  HardDrive,
  Download,
  Trash2,
  Archive,
  AlertTriangle,
} from "lucide-react";
import { useAppStore } from "../../store/appStore";
import { createBackup, listBackups, restoreBackup } from "../../lib/tauri";
import type { BackupInfo } from "../../lib/types";
import GameLocationPicker from "../setup/GameLocationPicker";
import { useModStore } from "../../store/modStore";

export default function SettingsPage() {
  const gameStatus = useAppStore((s) => s.gameStatus);
  const addToast = useAppStore((s) => s.addToast);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupsLoaded, setBackupsLoaded] = useState(false);
  const [isCreatingBackup, setIsCreatingBackup] = useState(false);

  const loadBackups = async () => {
    try {
      const data = await listBackups();
      setBackups(data);
      setBackupsLoaded(true);
    } catch {
      addToast({ type: "error", message: "Could not load backups." });
    }
  };

  const handleCreateBackup = async () => {
    setIsCreatingBackup(true);
    try {
      await createBackup();
      addToast({ type: "success", message: "Backup created." });
      await loadBackups();
    } catch (err) {
      addToast({ type: "error", message: `Backup failed: ${err}` });
    } finally {
      setIsCreatingBackup(false);
    }
  };

  const handleRestore = async (filename: string) => {
    try {
      await restoreBackup(filename);
      addToast({ type: "success", message: "Backup restored." });
    } catch (err) {
      addToast({ type: "error", message: `Restore failed: ${err}` });
    }
  };

  return (
    <div className="max-w-2xl space-y-6">
      {/* Game Info */}
      <section className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-5">
        <h3 className="text-base font-semibold text-[var(--color-text-primary)] mb-4 flex items-center gap-2">
          <HardDrive size={18} />
          Game Information
        </h3>
        <div className="space-y-3 text-sm">
          <div className="flex items-center justify-between">
            <span className="text-[var(--color-text-secondary)]">Status</span>
            <span
              className={`font-medium ${
                gameStatus?.installed
                  ? "text-[var(--color-success)]"
                  : "text-[var(--color-error)]"
              }`}
            >
              {gameStatus?.installed ? "Installed" : "Not Found"}
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[var(--color-text-secondary)]">
              Game Path
            </span>
            <span className="text-[var(--color-text-primary)] font-mono text-xs max-w-[300px] truncate">
              {gameStatus?.game_path ?? "N/A"}
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[var(--color-text-secondary)]">BepInEx</span>
            <span
              className={`font-medium ${
                gameStatus?.bepinex_installed
                  ? "text-[var(--color-success)]"
                  : "text-[var(--color-warning)]"
              }`}
            >
              {gameStatus?.bepinex_installed ? "Installed" : "Not Installed"}
            </span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-[var(--color-text-secondary)]">
              Active Profile
            </span>
            <span className="text-[var(--color-text-primary)] font-medium">
              {gameStatus?.active_profile ?? "Default"}
            </span>
          </div>
        </div>
        <GameLocationPicker onSelected={status => {
          useAppStore.getState().setGameStatus(status);
          useModStore.setState({ installedMods: [], packages: [], selectedPackage: null, packageError: null });
          useAppStore.getState().setInitialized(false);
        }} />
      </section>

      {/* Paths */}
      <section className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-5">
        <h3 className="text-base font-semibold text-[var(--color-text-primary)] mb-4 flex items-center gap-2">
          <FolderOpen size={18} />
          Data Locations
        </h3>
        <div className="space-y-2 text-sm text-[var(--color-text-secondary)]">
          <p>
            Active mod files are in the game's BepInEx folder. Saved profiles are in ~/Library/Application Support/com.macheim/profiles.
          </p>
          <p className="font-mono text-xs text-[var(--color-text-muted)] bg-[var(--color-bg-input)] px-3 py-2 rounded-md">
            {gameStatus?.game_path
              ? `${gameStatus.game_path.replace(/\/[^/]+\.app\/?$/, "")}/BepInEx/`
              : "~/Library/Application Support/Steam/steamapps/common/Valheim/BepInEx/"}
          </p>
        </div>
      </section>

      <p className="text-sm text-[var(--color-text-secondary)]">Backups contain profile metadata and configuration—not mod binaries or worlds. Restoring creates a separate profile. Thunderstore mods must be downloaded again; keep a separate copy of manual mods.</p>
      {/* Backups */}
      <section className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-5">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-base font-semibold text-[var(--color-text-primary)] flex items-center gap-2">
            <Archive size={18} />
            Backups
          </h3>
          <div className="flex gap-2">
            {!backupsLoaded && (
              <button
                onClick={loadBackups}
                className="text-xs px-3 py-1.5 rounded-md border border-[var(--color-border-default)] text-[var(--color-text-secondary)]
                  hover:bg-[var(--color-bg-elevated)] transition-colors cursor-pointer"
              >
                Load Backups
              </button>
            )}
            <button
              onClick={handleCreateBackup}
              disabled={isCreatingBackup}
              className="text-xs px-3 py-1.5 rounded-md bg-[var(--color-accent-primary)] text-white
                hover:bg-[var(--color-accent-primary-hover)] disabled:opacity-50 transition-colors cursor-pointer"
            >
              {isCreatingBackup ? "Creating..." : "Create Backup"}
            </button>
          </div>
        </div>
        {backupsLoaded && backups.length === 0 && (
          <p className="text-sm text-[var(--color-text-muted)]">
            No backups found.
          </p>
        )}
        {backups.length > 0 && (
          <div className="space-y-2">
            {backups.map((b) => (
              <div
                key={b.filename}
                className="flex items-center justify-between p-3 rounded-md bg-[var(--color-bg-input)] text-sm"
              >
                <div>
                  <p className="text-[var(--color-text-primary)] font-medium">
                    {b.profile_name}
                  </p>
                  <p className="text-xs text-[var(--color-text-muted)]">
                    {(b.size / 1024).toFixed(1)} KB &middot;{" "}
                    {new Date(b.created_at).toLocaleDateString()}
                  </p>
                </div>
                <button
                  onClick={() => handleRestore(b.filename)}
                  className="text-xs px-3 py-1.5 rounded-md border border-[var(--color-border-default)] text-[var(--color-text-secondary)]
                    hover:bg-[var(--color-bg-elevated)] transition-colors cursor-pointer"
                >
                  <Download size={14} />
                </button>
              </div>
            ))}
          </div>
        )}
      </section>

      {/* Reserved action: do not present an inert destructive control as working. */}
      <section className="rounded-lg border border-[var(--color-error)]/30 bg-[var(--color-error)]/5 p-5">
        <h3 className="text-base font-semibold text-[var(--color-error)] mb-3 flex items-center gap-2">
          <AlertTriangle size={18} />
          Danger Zone
        </h3>
        <p className="text-sm text-[var(--color-text-secondary)] mb-4">
          These actions are destructive and cannot be undone. Please create a
          backup first.
        </p>
        <button
          disabled
          title="Not available; manage individual mods in Installed Mods"
          className="text-xs px-4 py-2 rounded-md border border-[var(--color-error)]/50 text-[var(--color-error)]
            hover:bg-[var(--color-error)]/10 transition-colors cursor-pointer flex items-center gap-2"
        >
          <Trash2 size={14} />
          Remove All Mods
        </button>
      </section>
    </div>
  );
}
