import { RefreshCw } from "lucide-react";
import { useAppStore } from "../../store/appStore";
import { useProfileStore } from "../../store/profileStore";

const pageTitles: Record<string, string> = {
  browse: "Browse Mods",
  installed: "Installed Mods",
  modpacks: "Modpacks",
  config: "Config Editor",
  compatibility: "Mac Compatibility",
  profiles: "Profiles",
  settings: "Settings",
  setup: "Setup",
};

interface HeaderProps {
  onRefresh?: () => void;
  isRefreshing?: boolean;
}

export default function Header({ onRefresh, isRefreshing }: HeaderProps) {
  const currentPage = useAppStore((s) => s.currentPage);
  const catalog = useProfileStore(s => s.profiles.find(p => p.name === s.activeProfile)?.catalog_source ?? "thunderstore");

  return (
    <header className="h-14 shrink-0 flex items-center gap-4 px-6 border-b border-[var(--color-border-subtle)] bg-[var(--color-bg-primary)]">
      <h2 className="text-lg font-semibold text-[var(--color-text-primary)] whitespace-nowrap">
        {pageTitles[currentPage] ?? "Macheim"}
      </h2>

      <div className="flex-1" />
      {(currentPage === "browse" || currentPage === "modpacks") && <span className="text-xs text-[var(--color-text-muted)]">{catalog === "hexium" ? "Hexium" : "Thunderstore"} catalog</span>}

      {onRefresh && (
        <button
          onClick={onRefresh}
          disabled={isRefreshing}
          className="flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm
            text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]
            hover:bg-[var(--color-bg-card)] transition-colors disabled:opacity-50 cursor-pointer"
          title="Refresh"
        >
          <RefreshCw
            size={16}
            className={isRefreshing ? "animate-spin" : ""}
          />
          {isRefreshing ? "Refreshing..." : "Refresh"}
        </button>
      )}
    </header>
  );
}
