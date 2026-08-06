import { useEffect, useState } from "react";
import {
  Package,
  Trash2,
  Search,
  Power,
  PowerOff,
  RefreshCw,
  Loader2,
} from "lucide-react";
import { ListSkeleton } from "../common/LoadingSkeleton";
import { useModStore } from "../../store/modStore";
import { useAppStore } from "../../store/appStore";
import {
  getInstalledMods,
  toggleMod,
  uninstallMod,
  syncMods,
  listUnmanagedMods,
} from "../../lib/tauri";

export default function InstalledModList() {
  const installedMods = useModStore((s) => s.installedMods);
  const setInstalledMods = useModStore((s) => s.setInstalledMods);
  const isLoading = useModStore((s) => s.isLoadingInstalled);
  const setLoading = useModStore((s) => s.setLoadingInstalled);
  const addToast = useAppStore((s) => s.addToast);
  const [localSearch, setLocalSearch] = useState("");
  const [togglingMod, setTogglingMod] = useState<string | null>(null);
  const [uninstallingMod, setUninstallingMod] = useState<string | null>(null);
  const [syncing, setSyncing] = useState(false);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      try {
        const mods = await getInstalledMods();
        if (!cancelled) setInstalledMods(mods);
      } catch (err) {
        if (!cancelled) {
          addToast({
            type: "error",
            message: `Failed to load installed mods: ${err}`,
          });
        }
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    load();
    return () => {
      cancelled = true;
    };
  }, [setInstalledMods, setLoading, addToast]);

  const handleToggle = async (fullName: string, currentEnabled: boolean) => {
    setTogglingMod(fullName);
    try {
      await toggleMod(fullName, !currentEnabled);
      setInstalledMods(
        installedMods.map((m) =>
          m.full_name === fullName ? { ...m, enabled: !currentEnabled } : m
        )
      );
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to toggle mod: ${err}`,
      });
    } finally {
      setTogglingMod(null);
    }
  };

  const handleUninstall = async (fullName: string, name: string) => {
    setUninstallingMod(fullName);
    try {
      await uninstallMod(fullName);
      setInstalledMods(installedMods.filter((m) => m.full_name !== fullName));
      addToast({ type: "info", message: `Uninstalled ${name}` });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to uninstall ${name}: ${err}`,
      });
    } finally {
      setUninstallingMod(null);
    }
  };

  const filtered = localSearch.trim()
    ? installedMods.filter((m) => {
        const q = localSearch.toLowerCase();
        return (
          m.name.toLowerCase().includes(q) ||
          m.full_name.toLowerCase().includes(q) ||
          m.author.toLowerCase().includes(q)
        );
      })
    : installedMods;

  const enabledCount = installedMods.filter((m) => m.enabled).length;
  const disabledCount = installedMods.length - enabledCount;

  if (isLoading && installedMods.length === 0) {
    return <ListSkeleton rows={8} />;
  }

  if (installedMods.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <Package
          size={48}
          className="text-[var(--color-text-muted)] mb-4"
        />
        <h3 className="text-lg font-semibold text-[var(--color-text-secondary)] mb-1">
          No mods installed
        </h3>
        <p className="text-sm text-[var(--color-text-muted)]">
          Go to Browse Mods or Modpacks to install some.
        </p>
      </div>
    );
  }

  return (
    <div>
      {/* Search + Stats */}
      <div className="flex flex-col gap-4 mb-6">
        <div className="relative">
          <Search
            size={18}
            className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[var(--color-text-muted)] pointer-events-none"
          />
          <input
            type="text"
            value={localSearch}
            onChange={(e) => setLocalSearch(e.target.value)}
            placeholder="Search installed mods..."
            className="w-full pl-10 pr-4 py-2.5 rounded-xl text-sm
              bg-[var(--color-bg-input)] border border-[var(--color-border-default)]
              text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)]
              focus:outline-none focus:border-[var(--color-accent-primary)] focus:ring-1 focus:ring-[var(--color-accent-primary)]/30
              transition-colors"
          />
        </div>

        <div className="flex items-center gap-4 text-xs text-[var(--color-text-muted)]">
          <span className="font-medium text-[var(--color-text-secondary)]">
            {installedMods.length} mods total
          </span>
          <span className="flex items-center gap-1 text-[var(--color-success)]">
            <Power size={12} />
            {enabledCount} enabled
          </span>
          {disabledCount > 0 && (
            <span className="flex items-center gap-1 text-[var(--color-text-muted)]">
              <PowerOff size={12} />
              {disabledCount} disabled
            </span>
          )}

          <div className="flex-1" />

          <button
            onClick={async () => {
              setSyncing(true);
              try {
                // Check for unmanaged mods before cleaning
                const unmanaged = await listUnmanagedMods();
                let doClean = false;
                if (unmanaged.length > 0) {
                  doClean = window.confirm(
                    `The following ${unmanaged.length} mod(s) are not managed by Macheim and will be removed:\n\n` +
                    unmanaged.join("\n") +
                    "\n\nProceed with cleanup?"
                  );
                }
                const result = await syncMods(doClean);
                const msgs: string[] = [];
                if (result.reinstalled.length > 0) msgs.push(`${result.reinstalled.length} reinstalled`);
                if (result.cleaned.length > 0) msgs.push(`${result.cleaned.length} cleaned`);
                if (result.failed.length > 0) msgs.push(`${result.failed.length} failed`);
                addToast({
                  type: result.failed.length > 0 ? "warning" : "success",
                  message: `Sync complete: ${msgs.join(", ") || "all up to date"}`,
                });
                const mods = await getInstalledMods();
                setInstalledMods(mods);
              } catch (err) {
                addToast({ type: "error", message: `Sync failed: ${err}` });
              } finally {
                setSyncing(false);
              }
            }}
            disabled={syncing}
            className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-medium
              bg-[var(--color-accent-amber)] text-white hover:bg-[var(--color-accent-amber-hover)]
              transition-all cursor-pointer disabled:opacity-60"
          >
            {syncing ? <Loader2 size={12} className="animate-spin" /> : <RefreshCw size={12} />}
            {syncing ? "Syncing..." : "Sync & Clean"}
          </button>
        </div>
      </div>

      {/* Mod List */}
      <div className="space-y-2">
        {filtered.map((mod) => (
          <div
            key={mod.full_name}
            className={`flex items-center gap-4 p-3.5 rounded-xl border transition-all
              ${
                mod.enabled
                  ? "border-[var(--color-border-subtle)] bg-[var(--color-bg-card)]"
                  : "border-[var(--color-border-subtle)] bg-[var(--color-bg-card)] opacity-50"
              }
            `}
          >
            {mod.icon ? (
              <img
                src={mod.icon}
                alt={mod.name}
                className="w-10 h-10 rounded-lg shrink-0 bg-[var(--color-bg-input)] object-cover"
                loading="lazy"
              />
            ) : (
              <div className="w-10 h-10 rounded-lg shrink-0 bg-[var(--color-bg-input)] flex items-center justify-center">
                <Package
                  size={18}
                  className="text-[var(--color-text-muted)]"
                />
              </div>
            )}

            <div className="flex-1 min-w-0">
              <div className="flex items-center gap-2">
                <h4 className="text-sm font-semibold text-[var(--color-text-primary)] truncate">
                  {mod.name}
                </h4>
                <span className="text-xs text-[var(--color-text-muted)] font-mono shrink-0">
                  v{mod.version}
                </span>
                {!mod.enabled && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--color-text-muted)]/15 text-[var(--color-text-muted)] font-medium">
                    DISABLED
                  </span>
                )}
              </div>
              <p className="text-xs text-[var(--color-text-muted)] truncate">
                by {mod.author}
              </p>
            </div>

            {/* Toggle */}
            <button
              onClick={() => handleToggle(mod.full_name, mod.enabled)}
              disabled={togglingMod === mod.full_name}
              className={`relative w-11 h-6 rounded-full shrink-0 transition-colors cursor-pointer
                ${
                  mod.enabled
                    ? "bg-[var(--color-success)]"
                    : "bg-[var(--color-border-default)]"
                }
              `}
              title={mod.enabled ? "Disable mod" : "Enable mod"}
            >
              <div
                className={`absolute top-0.5 w-5 h-5 rounded-full bg-white shadow-sm transition-transform
                  ${mod.enabled ? "translate-x-[22px]" : "translate-x-0.5"}
                `}
              />
            </button>

            {/* Uninstall */}
            <button
              onClick={() => handleUninstall(mod.full_name, mod.name)}
              disabled={uninstallingMod === mod.full_name}
              className="p-2 rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-error)] hover:bg-[var(--color-error)]/10
                transition-colors disabled:opacity-50 cursor-pointer"
              title="Uninstall"
            >
              <Trash2 size={16} />
            </button>
          </div>
        ))}

        {filtered.length === 0 && localSearch.trim() && (
          <div className="text-center py-10 text-sm text-[var(--color-text-muted)]">
            No mods matching "{localSearch}"
          </div>
        )}
      </div>
    </div>
  );
}
