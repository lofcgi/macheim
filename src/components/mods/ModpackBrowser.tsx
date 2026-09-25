import { useEffect, useState } from "react";
import {
  Layers,
  Download,
  CheckCircle,
  Loader2,
  Package,
  Search,
  Flame,
  Clock,
  Star,
  ArrowDownAZ,
} from "lucide-react";
import { GridSkeleton } from "../common/LoadingSkeleton";
import { useModStore } from "../../store/modStore";
import { useAppStore } from "../../store/appStore";
import { installModpack, getInstalledMods } from "../../lib/tauri";
import { loadCatalog } from "../../lib/catalog";
import CatalogStatus from "./CatalogStatus";

type ModpackSort = "popular" | "updated" | "rated" | "name";

function formatDownloads(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return String(n);
}

function formatDate(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleDateString("en-US", {
      month: "short",
      day: "numeric",
      year: "numeric",
    });
  } catch {
    return dateStr;
  }
}

const sortTabs: { value: ModpackSort; label: string; icon: typeof Flame }[] = [
  { value: "popular", label: "Popular", icon: Flame },
  { value: "updated", label: "Newest", icon: Clock },
  { value: "rated", label: "Top Rated", icon: Star },
  { value: "name", label: "A-Z", icon: ArrowDownAZ },
];

export default function ModpackBrowser() {
  const packages = useModStore((s) => s.packages);
  const isLoading = useModStore((s) => s.isLoadingPackages);
  const installedMods = useModStore((s) => s.installedMods);
  const isInstallingMod = useModStore((s) => s.isInstallingMod);
  const setInstallingMod = useModStore((s) => s.setInstallingMod);
  const setInstalledMods = useModStore((s) => s.setInstalledMods);
  const setSelectedPackage = useModStore((s) => s.setSelectedPackage);
  const addToast = useAppStore((s) => s.addToast);

  const [localSearch, setLocalSearch] = useState("");
  const [sortBy, setSortBy] = useState<ModpackSort>("popular");

  useEffect(() => {
    if (useModStore.getState().packages.length === 0) void loadCatalog();
  }, []);

  // Filter modpacks
  const modpacks = packages.filter((pkg) => {
    if (pkg.is_deprecated) return false;
    const cats = (pkg.categories ?? []).map((c) => c.toLowerCase());
    const nameL = pkg.name.toLowerCase();
    const descL = (pkg.description ?? "").toLowerCase();
    return (
      cats.includes("modpacks") ||
      nameL.includes("modpack") ||
      nameL.includes("mod pack") ||
      descL.includes("modpack")
    );
  });

  // Search
  const searched = localSearch.trim()
    ? modpacks.filter((pkg) => {
        const q = localSearch.toLowerCase();
        return (
          pkg.name.toLowerCase().includes(q) ||
          pkg.owner.toLowerCase().includes(q) ||
          (pkg.description ?? "").toLowerCase().includes(q)
        );
      })
    : modpacks;

  // Sort
  const sorted = [...searched].sort((a, b) => {
    switch (sortBy) {
      case "popular":
        return (b.downloads ?? 0) - (a.downloads ?? 0);
      case "updated":
        return new Date(b.date_updated).getTime() - new Date(a.date_updated).getTime();
      case "rated":
        return b.rating_score - a.rating_score;
      case "name":
        return a.name.localeCompare(b.name);
      default:
        return 0;
    }
  });

  const handleInstall = async (fullName: string, version: string, name: string) => {
    setInstallingMod(fullName);
    try {
      await installModpack(fullName, version);
      const mods = await getInstalledMods();
      setInstalledMods(mods);
      addToast({ type: "success", message: `Installed modpack ${name}` });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to install ${name}: ${err}`,
      });
    } finally {
      setInstallingMod(null);
    }
  };

  if (isLoading && packages.length === 0) {
    return <GridSkeleton count={6} />;
  }

  return (
    <div>
      <CatalogStatus />
      {/* Search + Sort Bar */}
      <div className="flex flex-col gap-4 mb-6">
        {/* Search */}
        <div className="relative">
          <Search
            size={18}
            className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[var(--color-text-muted)] pointer-events-none"
          />
          <input
            type="text"
            value={localSearch}
            onChange={(e) => setLocalSearch(e.target.value)}
            placeholder="Search modpacks..."
            className="w-full pl-10 pr-4 py-2.5 rounded-xl text-sm
              bg-[var(--color-bg-input)] border border-[var(--color-border-default)]
              text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)]
              focus:outline-none focus:border-[var(--color-accent-primary)] focus:ring-1 focus:ring-[var(--color-accent-primary)]/30
              transition-colors"
          />
        </div>

        {/* Sort Tabs */}
        <div className="flex items-center gap-2">
          {sortTabs.map((tab) => {
            const Icon = tab.icon;
            const isActive = sortBy === tab.value;
            return (
              <button
                key={tab.value}
                onClick={() => setSortBy(tab.value)}
                className={`flex items-center gap-1.5 px-4 py-2 rounded-lg text-sm font-medium transition-all cursor-pointer
                  ${
                    isActive
                      ? "bg-[var(--color-accent-amber)] text-white shadow-sm shadow-orange-900/20"
                      : "bg-[var(--color-bg-card)] border border-[var(--color-border-subtle)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-default)]"
                  }
                `}
              >
                <Icon size={14} />
                {tab.label}
              </button>
            );
          })}

          <div className="flex-1" />
          <span className="text-xs text-[var(--color-text-muted)]">
            {sorted.length} modpacks
          </span>
        </div>
      </div>

      {/* Results */}
      {sorted.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <Layers
            size={48}
            className="text-[var(--color-text-muted)] mb-4"
          />
          <h3 className="text-lg font-semibold text-[var(--color-text-secondary)] mb-1">
            {localSearch ? "No matching modpacks" : "No modpacks found"}
          </h3>
          <p className="text-sm text-[var(--color-text-muted)]">
            {localSearch
              ? "Try a different search term."
              : "Modpacks will appear here when available on Thunderstore."}
          </p>
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {sorted.map((pkg) => {
            const isInstalled = installedMods.some(
              (m) => m.full_name === pkg.full_name
            );
            const isInstalling = isInstallingMod === pkg.full_name;

            return (
              <div
                key={pkg.full_name}
                onClick={() => setSelectedPackage(pkg)}
                className="rounded-xl border border-[var(--color-border-subtle)] bg-[var(--color-bg-card)]
                  hover:bg-[var(--color-bg-card-hover)] hover:border-[var(--color-border-default)]
                  transition-all duration-150 overflow-hidden cursor-pointer"
              >
                <div className="p-4">
                  <div className="flex items-start gap-3">
                    {pkg.icon ? (
                      <img
                        src={pkg.icon}
                        alt={pkg.name}
                        className="w-16 h-16 rounded-xl shrink-0 bg-[var(--color-bg-input)] object-cover"
                        loading="lazy"
                      />
                    ) : (
                      <div className="w-16 h-16 rounded-xl shrink-0 bg-[var(--color-bg-input)] flex items-center justify-center">
                        <Package
                          size={28}
                          className="text-[var(--color-text-muted)]"
                        />
                      </div>
                    )}
                    <div className="flex-1 min-w-0">
                      <h3 className="text-sm font-bold text-[var(--color-text-primary)] truncate">
                        {pkg.name}
                      </h3>
                      <p className="text-xs text-[var(--color-text-muted)] mt-0.5">
                        by {pkg.owner}
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
                    <span className="flex items-center gap-1">
                      <Star size={12} />
                      {pkg.rating_score}
                    </span>
                    <span>
                      {formatDate(pkg.date_updated)}
                    </span>
                  </div>

                  <button
                    onClick={() =>
                      handleInstall(pkg.full_name, pkg.version_number, pkg.name)
                    }
                    disabled={isInstalled || isInstalling}
                    className={`px-4 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer
                      ${
                        isInstalled
                          ? "bg-[var(--color-success)]/15 text-[var(--color-success)]"
                          : isInstalling
                            ? "bg-[var(--color-accent-primary)]/15 text-[var(--color-accent-primary)]"
                            : "bg-[var(--color-accent-amber)] text-white hover:bg-[var(--color-accent-amber-hover)] active:scale-95"
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
          })}
        </div>
      )}
    </div>
  );
}
