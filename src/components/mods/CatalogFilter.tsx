import { useModStore } from "../../store/modStore";

export default function CatalogFilter() {
  const source = useModStore(s => s.sourceFilter);
  const setSource = useModStore(s => s.setSourceFilter);
  return <div className="mb-4 text-sm">
    <label htmlFor="catalog-filter">Mod source </label>
    <select id="catalog-filter" value={source} onChange={e => setSource(e.target.value as typeof source)} className="p-2 rounded bg-[var(--color-bg-input)]">
      <option value="all">All sources</option>
      <option value="thunderstore">Thunderstore</option>
      <option value="hexium">Hexium</option>
    </select>
    <p className="mt-2 text-xs text-[var(--color-text-muted)]">Both sources install into this profile. Updates keep each mod's original source. New dependencies prefer the selected mod's source; dependencies only available on the other source use that source. Conflicting versions stop installation.</p>
  </div>;
}
