import { useCallback } from "react";
import Sidebar from "./Sidebar";
import Header from "./Header";
import ModGrid from "../mods/ModGrid";
import InstalledModList from "../mods/InstalledModList";
import ModpackBrowser from "../mods/ModpackBrowser";
import ConfigEditor from "../config/ConfigEditor";
import ProfileManager from "../profiles/ProfileManager";
import SettingsPage from "./SettingsPage";
import CompatibilityPage from "../compatibility/CompatibilityPage";
import { useProfileStore } from "../../store/profileStore";
import ModDetail from "../mods/ModDetail";
import { useAppStore } from "../../store/appStore";
import { useModStore } from "../../store/modStore";
import { getInstalledMods } from "../../lib/tauri";
import { loadCatalog } from "../../lib/catalog";

export default function MainLayout() {
  const activeProfile = useProfileStore(s => s.activeProfile);
  const currentPage = useAppStore((s) => s.currentPage);
  const addToast = useAppStore((s) => s.addToast);
  const setInstalledMods = useModStore((s) => s.setInstalledMods);
  const setLoadingInstalled = useModStore((s) => s.setLoadingInstalled);
  const isLoadingPackages = useModStore((s) => s.isLoadingPackages);
  const isLoadingInstalled = useModStore((s) => s.isLoadingInstalled);
  const selectedPackage = useModStore((s) => s.selectedPackage);
  const setSelectedPackage = useModStore((s) => s.setSelectedPackage);

  const handleRefresh = useCallback(async () => {
    if (currentPage === "browse" || currentPage === "modpacks") {
      await loadCatalog(true);
    } else if (currentPage === "installed") {
      setLoadingInstalled(true);
      try {
        const mods = await getInstalledMods();
        setInstalledMods(mods);
      } catch (err) {
        addToast({
          type: "error",
          message: `Failed to load installed mods: ${err}`,
        });
      } finally {
        setLoadingInstalled(false);
      }
    }
  }, [
    currentPage,
    setLoadingInstalled,
    setInstalledMods,
    addToast,
  ]);

  const showRefresh =
    currentPage === "browse" ||
    currentPage === "installed" ||
    currentPage === "modpacks";

  const isRefreshing = isLoadingPackages || isLoadingInstalled;

  const renderPage = () => {
    switch (currentPage) {
      case "browse":
        return <ModGrid />;
      case "installed":
        return <InstalledModList key={activeProfile} />;
      case "modpacks":
        return <ModpackBrowser />;
      case "config":
        return <ConfigEditor key={activeProfile} />;
      case "compatibility":
        return <CompatibilityPage key={activeProfile} />;
      case "profiles":
        return <ProfileManager />;
      case "settings":
        return <SettingsPage />;
      default:
        return <ModGrid />;
    }
  };

  return (
    <div className="flex h-screen w-screen overflow-hidden">
      <Sidebar />
      <div className="flex flex-col flex-1 min-w-0">
        <Header
          onRefresh={showRefresh ? handleRefresh : undefined}
          isRefreshing={isRefreshing}
        />
        <main className="flex-1 overflow-y-auto p-6">{renderPage()}</main>
      </div>

      {selectedPackage && (
        <ModDetail
          pkg={selectedPackage}
          onClose={() => setSelectedPackage(null)}
        />
      )}
    </div>
  );
}
