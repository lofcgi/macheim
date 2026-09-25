import { useEffect, useState } from "react";
import { Package, ChevronDown } from "lucide-react";
import ModCard from "./ModCard";
import ModSearch from "./ModSearch";
import { GridSkeleton } from "../common/LoadingSkeleton";
import { useModStore } from "../../store/modStore";
import { loadCatalog } from "../../lib/catalog";
import CatalogStatus from "./CatalogStatus";

const PAGE_SIZE = 48;

export default function ModGrid() {
  const packages = useModStore((s) => s.packages);
  const isLoading = useModStore((s) => s.isLoadingPackages);
  const getFilteredPackages = useModStore((s) => s.getFilteredPackages);
  const [displayCount, setDisplayCount] = useState(PAGE_SIZE);

  useEffect(() => {
    if (useModStore.getState().packages.length === 0) void loadCatalog();
  }, []);

  // Reset display count when search changes
  const searchQuery = useModStore((s) => s.searchQuery);
  useEffect(() => {
    setDisplayCount(PAGE_SIZE);
  }, [searchQuery]);

  const filtered = getFilteredPackages();
  const displayed = filtered.slice(0, displayCount);
  const hasMore = displayCount < filtered.length;

  if (isLoading && packages.length === 0) {
    return (
      <div>
        <ModSearch />
        <GridSkeleton count={9} />
      </div>
    );
  }

  return (
    <div>
      <ModSearch />

      <CatalogStatus />

      {filtered.length === 0 ? (
        <div className="flex flex-col items-center justify-center py-20 text-center">
          <Package
            size={48}
            className="text-[var(--color-text-muted)] mb-4"
          />
          <h3 className="text-lg font-semibold text-[var(--color-text-secondary)] mb-1">
            No mods found
          </h3>
          <p className="text-sm text-[var(--color-text-muted)]">
            Try adjusting your search or refresh the package list.
          </p>
        </div>
      ) : (
        <>
          <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            {displayed.map((pkg) => (
              <ModCard key={pkg.full_name} pkg={pkg} />
            ))}
          </div>

          {hasMore && (
            <div className="flex justify-center mt-6 mb-4">
              <button
                onClick={() => setDisplayCount((c) => c + PAGE_SIZE)}
                className="flex items-center gap-2 px-6 py-2.5 rounded-lg text-sm font-medium
                  bg-[var(--color-bg-card)] border border-[var(--color-border-default)]
                  text-[var(--color-text-secondary)] hover:text-[var(--color-text-primary)]
                  hover:border-[var(--color-border-hover)] transition-all cursor-pointer"
              >
                <ChevronDown size={16} />
                Load More ({(filtered.length - displayCount).toLocaleString()} remaining)
              </button>
            </div>
          )}
        </>
      )}
    </div>
  );
}
