import { useEffect } from "react";
import { useAppStore } from "./store/appStore";
import { useUpdateStore } from "./store/updateStore";
import { getGameStatus } from "./lib/tauri";
import SetupWizard from "./components/setup/SetupWizard";
import MainLayout from "./components/layout/MainLayout";
import UpdateBanner from "./components/layout/UpdateBanner";
import ToastContainer from "./components/common/Toast";
import ProgressOverlay from "./components/common/ProgressOverlay";

export default function App() {
  const gameStatus = useAppStore((s) => s.gameStatus);
  const isInitialized = useAppStore((s) => s.isInitialized);
  const setGameStatus = useAppStore((s) => s.setGameStatus);
  const setInitialized = useAppStore((s) => s.setInitialized);

  useEffect(() => {
    async function checkStatus() {
      try {
        const status = await getGameStatus();
        setGameStatus(status);
        if (status.installed && status.bepinex_installed) {
          setInitialized(true);
        }
      } catch {
        // Backend not ready; will show setup wizard
      }
    }
    checkStatus();
  }, [setGameStatus, setInitialized]);

  useEffect(() => {
    void useUpdateStore.getState().checkForUpdate();
  }, []);

  const needsSetup =
    !isInitialized ||
    !gameStatus?.installed ||
    !gameStatus?.bepinex_installed;

  return (
    <>
      <div className="flex flex-col h-screen w-screen">
        <UpdateBanner />
        <div className="flex-1 min-h-0">
          {needsSetup ? <SetupWizard /> : <MainLayout />}
        </div>
      </div>
      <ToastContainer />
      <ProgressOverlay />
    </>
  );
}
