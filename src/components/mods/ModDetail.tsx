import { useEffect, useState } from "react";
import {
  X,
  Download,
  CheckCircle,
  Loader2,
  ExternalLink,
  Package,
  Clock,
  Star,
  Layers,
  AlertTriangle,
} from "lucide-react";
import type { ThunderstorePackage, PackageDetail } from "../../lib/types";
import { useModStore } from "../../store/modStore";
import { useAppStore } from "../../store/appStore";
import { changeModVersion, uninstallMod, getInstalledMods, getPackageDetails } from "../../lib/tauri";
import { confirm } from "@tauri-apps/plugin-dialog";

interface ModDetailProps {
  pkg: ThunderstorePackage;
  onClose: () => void;
}

function formatDate(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}

function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export default function ModDetail({ pkg, onClose }: ModDetailProps) {
  const installedMods = useModStore((s) => s.installedMods);
  const isInstallingMod = useModStore((s) => s.isInstallingMod);
  const setInstallingMod = useModStore((s) => s.setInstallingMod);
  const setInstalledMods = useModStore((s) => s.setInstalledMods);
  const addToast = useAppStore((s) => s.addToast);

  const [detail, setDetail] = useState<PackageDetail | null>(null);
  const [loadingDetail, setLoadingDetail] = useState(true);
  const [detailError, setDetailError] = useState<string | null>(null);
  const [selectedVersion, setSelectedVersion] = useState(pkg.version_number);

  // Fetch full details on mount
  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoadingDetail(true);
      setDetailError(null);
      try {
        const d = await getPackageDetails(pkg.full_name);
        if (!cancelled) setDetail(d);
      } catch (err) {
        if (!cancelled) setDetailError(String(err));
      } finally {
        if (!cancelled) setLoadingDetail(false);
      }
    }
    load();
    return () => { cancelled = true; };
  }, [pkg.full_name]);

  const isInstalled = installedMods.some(
    (m) => m.full_name === pkg.full_name
  );
  const isInstalling = isInstallingMod === pkg.full_name;

  const latestVersion = detail?.versions?.find(v => v.version_number === selectedVersion);
  const dependencies = latestVersion?.dependencies?.filter(
    (d) => !d.startsWith("denikson-BepInExPack")
  ) ?? [];

  const handleInstall = async () => {
    if (isInstalling || !latestVersion) return;
    if (isInstalled && !await confirm(`Change ${pkg.name} to v${selectedVersion}? Keep the same versions as your server. Existing configuration and disabled state are preserved.`, { title: "Change mod version", kind: "warning" })) return;
    setInstallingMod(pkg.full_name);
    try {
      await changeModVersion(pkg.full_name, selectedVersion);
      const mods = await getInstalledMods();
      setInstalledMods(mods);
      addToast({ type: "success", message: `Installed ${pkg.name}` });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to install ${pkg.name}: ${err}`,
      });
    } finally {
      setInstallingMod(null);
    }
  };

  const handleUninstall = async () => {
    try {
      await uninstallMod(pkg.full_name);
      const mods = await getInstalledMods();
      setInstalledMods(mods);
      addToast({ type: "info", message: `Uninstalled ${pkg.name}` });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to uninstall ${pkg.name}: ${err}`,
      });
    }
  };

  const thunderstoreUrl = detail?.package_url ?? `https://thunderstore.io/c/valheim/p/${pkg.owner}/${pkg.name}/`;

  return (
    <>
      <div
        className="fixed inset-0 z-40 bg-black/60 backdrop-blur-sm animate-[fadeIn_0.15s_ease-out]"
        onClick={onClose}
      />

      <div className="fixed inset-y-0 right-0 z-50 w-full max-w-xl bg-[var(--color-bg-sidebar)] border-l border-[var(--color-border-default)] shadow-2xl animate-[slideInRight_0.2s_ease-out] flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-[var(--color-border-subtle)]">
          <h2 className="text-base font-semibold text-[var(--color-text-primary)]">
            Mod Details
          </h2>
          <button
            onClick={onClose}
            className="p-1.5 rounded-lg hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
          >
            <X size={18} className="text-[var(--color-text-muted)]" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5 space-y-6">
          {/* Top section - always visible from listing data */}
          <div className="flex items-start gap-4">
            {pkg.icon ? (
              <img
                src={pkg.icon}
                alt={pkg.name}
                className="w-20 h-20 rounded-xl shrink-0 bg-[var(--color-bg-input)] object-cover shadow-md"
              />
            ) : (
              <div className="w-20 h-20 rounded-xl shrink-0 bg-[var(--color-bg-input)] flex items-center justify-center">
                <Package size={32} className="text-[var(--color-text-muted)]" />
              </div>
            )}
            <div className="min-w-0">
              <h3 className="text-xl font-bold text-[var(--color-text-primary)]">
                {pkg.name}
              </h3>
              <p className="text-sm text-[var(--color-text-secondary)] mt-0.5">
                by {pkg.owner}
              </p>
              <div className="flex items-center gap-4 mt-2.5 text-xs text-[var(--color-text-muted)]">
                <span className="flex items-center gap-1">
                  <Download size={13} />
                  {formatDownloads(pkg.downloads)}
                </span>
                <span className="flex items-center gap-1">
                  <Star size={13} />
                  {pkg.rating_score}
                </span>
                <span className="flex items-center gap-1">
                  <Clock size={13} />
                  {formatDate(pkg.date_updated)}
                </span>
              </div>
            </div>
          </div>

          {/* Description */}
          <div>
            <h4 className="text-xs font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">
              Description
            </h4>
            <p className="text-sm text-[var(--color-text-secondary)] leading-relaxed whitespace-pre-wrap">
              {pkg.description || "No description available."}
            </p>
          </div>

          {/* Categories */}
          {pkg.categories && pkg.categories.length > 0 && (
            <div>
              <h4 className="text-xs font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">
                Categories
              </h4>
              <div className="flex flex-wrap gap-1.5">
                {pkg.categories.map((cat) => (
                  <span
                    key={cat}
                    className="inline-block px-2.5 py-1 rounded-full text-xs bg-[var(--color-accent-primary)]/10 text-[var(--color-accent-primary)] border border-[var(--color-accent-primary)]/20"
                  >
                    {cat}
                  </span>
                ))}
              </div>
            </div>
          )}

          {/* Loading detail */}
          {loadingDetail && (
            <div className="flex items-center justify-center py-8">
              <Loader2 size={24} className="text-[var(--color-accent-primary)] animate-spin" />
              <span className="ml-2 text-sm text-[var(--color-text-muted)]">Loading details...</span>
            </div>
          )}

          {detailError && (
            <div className="flex items-start gap-2 p-3 rounded-lg bg-[var(--color-warning)]/10 border border-[var(--color-warning)]/20">
              <AlertTriangle size={16} className="text-[var(--color-warning)] mt-0.5 shrink-0" />
              <p className="text-xs text-[var(--color-text-secondary)]">
                Could not load full details: {detailError}
              </p>
            </div>
          )}

          {/* Version History */}
          {detail && detail.versions.length > 0 && (
            <div>
              <label htmlFor="mod-version">Version to install</label>
              <select id="mod-version" value={selectedVersion} onChange={e => setSelectedVersion(e.target.value)} className="w-full my-2 p-2 rounded bg-[var(--color-bg-input)]">
                {detail.versions.map(v => <option key={v.version_number} value={v.version_number}>{v.version_number}</option>)}
              </select>
              <button onClick={handleInstall} disabled={isInstalling || installedMods.some(m => m.full_name === pkg.full_name && m.version === selectedVersion)} className="mb-4 underline disabled:opacity-40">
                {isInstalling ? "Installing..." : `Install v${selectedVersion}`}
              </button>
              <h4 className="text-xs font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">
                Version History ({detail.versions.length})
              </h4>
              <div className="space-y-1.5 max-h-48 overflow-y-auto pr-1">
                {detail.versions.slice(0, 15).map((v, i) => (
                  <div
                    key={v.version_number}
                    className={`flex items-center justify-between px-3 py-2 rounded-lg text-sm
                      ${i === 0
                        ? "bg-[var(--color-accent-primary)]/10 border border-[var(--color-accent-primary)]/20"
                        : "bg-[var(--color-bg-input)]"
                      }
                    `}
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-[var(--color-text-primary)] font-mono text-xs font-medium">
                        v{v.version_number}
                      </span>
                      {i === 0 && (
                        <span className="px-1.5 py-0.5 rounded text-[10px] font-semibold bg-[var(--color-accent-primary)] text-white">
                          LATEST
                        </span>
                      )}
                    </div>
                    <div className="flex items-center gap-3 text-xs text-[var(--color-text-muted)]">
                      <span>{formatDownloads(v.downloads)}</span>
                      <span>{formatDate(v.date_created)}</span>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Dependencies */}
          {dependencies.length > 0 && (
            <div>
              <h4 className="text-xs font-semibold text-[var(--color-text-muted)] uppercase tracking-wider mb-2">
                <span className="flex items-center gap-1.5">
                  <Layers size={13} />
                  Dependencies ({dependencies.length})
                </span>
              </h4>
              <div className="flex flex-wrap gap-1.5 max-h-48 overflow-y-auto">
                {dependencies.map((dep) => {
                  const parts = dep.split("-");
                  const depName = parts.length >= 3
                    ? parts.slice(0, -1).join("-")
                    : dep;
                  const depVersion = parts.length >= 3
                    ? parts[parts.length - 1]
                    : "";
                  return (
                    <span
                      key={dep}
                      className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md text-xs bg-[var(--color-bg-input)] text-[var(--color-text-secondary)] border border-[var(--color-border-subtle)]"
                    >
                      {depName}
                      {depVersion && (
                        <span className="text-[var(--color-text-muted)]">
                          {depVersion}
                        </span>
                      )}
                    </span>
                  );
                })}
              </div>
            </div>
          )}

          {/* Thunderstore Link */}
          <a
            href={thunderstoreUrl}
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1.5 text-sm text-[var(--color-accent-primary)] hover:text-[var(--color-accent-primary-hover)] transition-colors"
          >
            <ExternalLink size={14} />
            View on source website
          </a>
        </div>

        {/* Action Footer */}
        <div className="px-5 py-4 border-t border-[var(--color-border-subtle)] flex gap-3">
          {isInstalled ? (
            <>
              <button
                className="flex-1 px-4 py-2.5 rounded-lg text-sm font-medium bg-[var(--color-success)]/15 text-[var(--color-success)] flex items-center justify-center gap-2"
                disabled
              >
                <CheckCircle size={16} />
                Installed (v
                {installedMods.find((m) => m.full_name === pkg.full_name)
                  ?.version ?? pkg.version_number}
                )
              </button>
              <button
                onClick={handleUninstall}
                className="px-4 py-2.5 rounded-lg text-sm font-medium border border-[var(--color-error)]/40 text-[var(--color-error)]
                  hover:bg-[var(--color-error)]/10 transition-colors cursor-pointer"
              >
                Uninstall
              </button>
            </>
          ) : (
            <button
              onClick={handleInstall}
              disabled={isInstalling}
              className="flex-1 px-4 py-2.5 rounded-lg text-sm font-semibold
                bg-[var(--color-accent-primary)] text-white
                hover:bg-[var(--color-accent-primary-hover)]
                disabled:opacity-60 active:scale-[0.98] transition-all cursor-pointer
                flex items-center justify-center gap-2"
            >
              {isInstalling ? (
                <>
                  <Loader2 size={16} className="animate-spin" />
                  Installing...
                </>
              ) : (
                <>
                  <Download size={16} />
                  Install v{pkg.version_number}
                </>
              )}
            </button>
          )}
        </div>

        <style>{`
          @keyframes fadeIn {
            from { opacity: 0; }
            to { opacity: 1; }
          }
          @keyframes slideInRight {
            from { transform: translateX(100%); }
            to { transform: translateX(0); }
          }
        `}</style>
      </div>
    </>
  );
}
