import { Search, Flame, Clock, Star, ArrowDownAZ } from "lucide-react";
import { useModStore } from "../../store/modStore";
import type { SortOption } from "../../lib/types";
import CatalogFilter from "./CatalogFilter";

const sortTabs: { value: SortOption; label: string; icon: typeof Flame }[] = [
  { value: "downloads", label: "Popular", icon: Flame },
  { value: "updated", label: "Newest", icon: Clock },
  { value: "rating", label: "Top Rated", icon: Star },
  { value: "name", label: "A-Z", icon: ArrowDownAZ },
];

export default function ModSearch() {
  const sortBy = useModStore((s) => s.sortBy);
  const setSortBy = useModStore((s) => s.setSortBy);
  const searchQuery = useModStore((s) => s.searchQuery);
  const setSearchQuery = useModStore((s) => s.setSearchQuery);

  return (
    <div className="flex flex-col gap-4 mb-6">
      <CatalogFilter />
      {/* Search */}
      <div className="relative">
        <Search
          size={18}
          className="absolute left-3.5 top-1/2 -translate-y-1/2 text-[var(--color-text-muted)] pointer-events-none"
        />
        <input
          type="text"
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          placeholder="Search mods..."
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
                    ? "bg-[var(--color-accent-primary)] text-white shadow-sm"
                    : "bg-[var(--color-bg-card)] border border-[var(--color-border-subtle)] text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)] hover:border-[var(--color-border-default)]"
                }
              `}
            >
              <Icon size={14} />
              {tab.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
