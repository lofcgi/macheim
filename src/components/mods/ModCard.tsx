import { Download, CheckCircle, Loader2, Package } from "lucide-react";
import type { ThunderstorePackage } from "../../lib/types";
import { useModStore } from "../../store/modStore";
import { useAppStore } from "../../store/appStore";
import { installMod, getInstalledMods } from "../../lib/tauri";

interface ModCardProps {
  pkg: ThunderstorePackage;
}

function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

export default function ModCard({ pkg }: ModCardProps) {
  const installedMods = useModStore((s) => s.installedMods);
  const isInstallingMod = useModStore((s) => s.isInstallingMod);
  const setInstallingMod = useModStore((s) => s.setInstallingMod);
  const setInstalledMods = useModStore((s) => s.setInstalledMods);
  const setSelectedPackage = useModStore((s) => s.setSelectedPackage);
  const addToast = useAppStore((s) => s.addToast);

  const isInstalled = installedMods.some(
    (m) => m.full_name === pkg.full_name
  );
  const isInstalling = isInstallingMod === pkg.full_name;

  const handleInstall = async (e: React.MouseEvent) => {
    e.stopPropagation();
    if (isInstalled || isInstalling) return;

    setInstallingMod(pkg.full_name);
    try {
      await installMod(pkg.full_name, pkg.version_number, pkg.source);
      // Refresh installed mods list
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

  return (
    <div
      onClick={() => setSelectedPackage(pkg)}
      className="group rounded-lg border border-[var(--color-border-subtle)] bg-[var(--color-bg-card)]
        hover:bg-[var(--color-bg-card-hover)] hover:border-[var(--color-border-default)]
        transition-all duration-150 cursor-pointer overflow-hidden"
    >
      <div className="p-4">
        <div className="flex items-start gap-3">
          {pkg.icon ? (
            <img
              src={pkg.icon}
              alt={pkg.name}
              className="w-14 h-14 rounded-lg shrink-0 bg-[var(--color-bg-input)] object-cover"
              loading="lazy"
            />
          ) : (
            <div className="w-14 h-14 rounded-lg shrink-0 bg-[var(--color-bg-input)] flex items-center justify-center">
              <Package
                size={24}
                className="text-[var(--color-text-muted)]"
              />
            </div>
          )}

          <div className="flex-1 min-w-0">
            <h3 className="text-sm font-semibold text-[var(--color-text-primary)] truncate group-hover:text-[var(--color-accent-amber)] transition-colors">
              {pkg.name}
            </h3>
            <p className="text-xs text-[var(--color-text-muted)] mt-0.5">
              by {pkg.owner} · {pkg.source === "hexium" ? "Hexium" : "Thunderstore"}
            </p>
            <p className="text-xs text-[var(--color-text-secondary)] mt-1.5 line-clamp-2 leading-relaxed">
              {pkg.description || "No description"}
            </p>
          </div>
        </div>
      </div>

      <div className="flex items-center justify-between px-4 py-2.5 border-t border-[var(--color-border-subtle)] bg-[var(--color-bg-primary)]/30">
        <div className="flex items-center gap-3 text-xs text-[var(--color-text-muted)]">
          <span className="flex items-center gap-1">
            <Download size={12} />
            {formatDownloads(pkg.downloads)}
          </span>
          <span>v{pkg.version_number}</span>
        </div>

        <button
          onClick={handleInstall}
          disabled={isInstalled || isInstalling}
          className={`
            px-3 py-1 rounded-md text-xs font-medium transition-all cursor-pointer
            ${
              isInstalled
                ? "bg-[var(--color-success)]/15 text-[var(--color-success)]"
                : isInstalling
                  ? "bg-[var(--color-accent-primary)]/15 text-[var(--color-accent-primary)]"
                  : "bg-[var(--color-accent-primary)] text-white hover:bg-[var(--color-accent-primary-hover)] active:scale-95"
            }
          `}
        >
          {isInstalled ? (
            <span className="flex items-center gap-1">
              <CheckCircle size={12} />
              Installed
            </span>
          ) : isInstalling ? (
            <span className="flex items-center gap-1">
              <Loader2 size={12} className="animate-spin" />
              Installing
            </span>
          ) : (
            "Install"
          )}
        </button>
      </div>
    </div>
  );
}
