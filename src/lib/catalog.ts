import { fetchPackages } from "./tauri";
import { useModStore } from "../store/modStore";

let pending: Promise<void> | null = null;

export function loadCatalog(forceRefresh = false): Promise<void> {
  if (pending) return pending;
  useModStore.setState({ isLoadingPackages: true, packageError: null });
  pending = (async () => {
    try {
      const packages = await fetchPackages(forceRefresh);
      useModStore.setState({ packages });
    } catch (err) {
      useModStore.setState({ packageError: String(err) });
    } finally {
      // The request owns loading state, not whichever page happens to be mounted.
      useModStore.setState({ isLoadingPackages: false });
      pending = null;
    }
  })();
  return pending;
}
