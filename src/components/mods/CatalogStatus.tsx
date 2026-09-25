import { useModStore } from "../../store/modStore";
import { loadCatalog } from "../../lib/catalog";

export default function CatalogStatus() {
  const error = useModStore(s => s.packageError);
  const loading = useModStore(s => s.isLoadingPackages);
  if (!error) return null;
  return <div role="alert" className="mb-4 rounded-lg border border-red-500/40 p-4 text-sm">
    <p>Could not refresh the mod catalog: {error}</p>
    <p>Your installed mods are unchanged. Check your connection, then retry.</p>
    <button disabled={loading} onClick={() => void loadCatalog(true)} className="mt-2 underline">Retry catalog download</button>
  </div>;
}
