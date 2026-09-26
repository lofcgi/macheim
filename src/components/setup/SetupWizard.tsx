import { useState, useEffect } from "react";
import {
  Search,
  CheckCircle,
  Loader2,
  XCircle,
  Shield,
  ArrowRight,
  AlertTriangle,
} from "lucide-react";
import ProgressBar from "../common/ProgressBar";
import { detectGame, installBepinex, getGameStatus } from "../../lib/tauri";
import { useAppStore } from "../../store/appStore";
import GameLocationPicker from "./GameLocationPicker";

type Step = "detect" | "bepinex" | "ready";

export default function SetupWizard() {
  const setGameStatus = useAppStore((s) => s.setGameStatus);
  const setInitialized = useAppStore((s) => s.setInitialized);

  const [step, setStep] = useState<Step>("detect");
  const [detecting, setDetecting] = useState(true);
  const [gamePath, setGamePath] = useState<string | null>(null);
  const [detectError, setDetectError] = useState<string | null>(null);
  const [installing, setInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState(0);
  const [installError, setInstallError] = useState<string | null>(null);

  // Step 1: Detect game on mount
  useEffect(() => {
    let cancelled = false;
    async function detect() {
      setDetecting(true);
      setDetectError(null);
      try {
        const status = await detectGame();
        if (cancelled) return;
        setGameStatus(status);
        setGamePath(status.game_path);

        if (status.installed && status.bepinex_installed) {
          setStep("ready");
        } else if (status.installed) {
          setStep("bepinex");
        }
      } catch (err) {
        if (cancelled) return;
        setDetectError(
          `Could not detect Valheim. Make sure it is installed via Steam. (${err})`
        );
      } finally {
        if (!cancelled) setDetecting(false);
      }
    }
    detect();
    return () => {
      cancelled = true;
    };
  }, [setGameStatus]);

  // Step 2: Install BepInEx
  const handleInstallBepinex = async () => {
    setInstalling(true);
    setInstallError(null);
    setInstallProgress(0);

    // Simulate progress steps while waiting for the install
    const interval = setInterval(() => {
      setInstallProgress((p) => {
        if (p >= 90) return 90;
        return p + Math.random() * 15;
      });
    }, 400);

    try {
      await installBepinex();
      clearInterval(interval);
      setInstallProgress(100);

      // Re-fetch status
      try {
        const status = await getGameStatus();
        setGameStatus(status);
      } catch {
        // Continue anyway
      }

      setTimeout(() => setStep("ready"), 500);
    } catch (err) {
      clearInterval(interval);
      setInstallError(`BepInEx installation failed: ${err}`);
      setInstallProgress(0);
    } finally {
      setInstalling(false);
    }
  };

  // Step 3: Done
  const handleFinish = async () => {
    try {
      const status = await getGameStatus();
      setGameStatus(status);
    } catch {
      // ok
    }
    setInitialized(true);
  };

  const handleRetryDetect = async () => {
    setDetecting(true);
    setDetectError(null);
    try {
      const status = await detectGame();
      setGameStatus(status);
      setGamePath(status.game_path);
      if (status.installed && status.bepinex_installed) {
        setStep("ready");
      } else if (status.installed) {
        setStep("bepinex");
      }
    } catch (err) {
      setDetectError(`Detection failed: ${err}`);
    } finally {
      setDetecting(false);
    }
  };

  return (
    <div className="h-screen w-screen flex items-center justify-center bg-[var(--color-bg-primary)]">
      <div className="w-full max-w-lg px-4">
        {/* Logo */}
        <div className="flex flex-col items-center mb-10">
          <div className="w-16 h-16 rounded-2xl bg-gradient-to-br from-[var(--color-accent-amber)] to-orange-700 flex items-center justify-center mb-4 shadow-lg shadow-orange-900/40">
            <Shield size={36} className="text-white" />
          </div>
          <h1 className="text-2xl font-bold text-[var(--color-text-primary)] tracking-tight">
            Macheim
          </h1>
          <p className="text-sm text-[var(--color-text-secondary)] mt-1">
            Valheim Mod Manager for macOS
          </p>
        </div>

        {/* Step indicator */}
        <div className="flex items-center justify-center gap-2 mb-8">
          {(["detect", "bepinex", "ready"] as Step[]).map((s, i) => (
            <div key={s} className="flex items-center gap-2">
              <div
                className={`w-2.5 h-2.5 rounded-full transition-colors ${
                  step === s
                    ? "bg-[var(--color-accent-amber)]"
                    : i <
                        ["detect", "bepinex", "ready"].indexOf(step)
                      ? "bg-[var(--color-success)]"
                      : "bg-[var(--color-border-default)]"
                }`}
              />
              {i < 2 && (
                <div className="w-12 h-px bg-[var(--color-border-default)]" />
              )}
            </div>
          ))}
        </div>

        {/* Card */}
        <div className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-6 shadow-xl shadow-black/30">
          {/* Step 1: Detect */}
          {step === "detect" && (
            <div className="text-center">
              {detecting ? (
                <>
                  <Loader2
                    size={40}
                    className="mx-auto text-[var(--color-accent-amber)] animate-spin mb-4"
                  />
                  <h2 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
                    Detecting Valheim...
                  </h2>
                  <p className="text-sm text-[var(--color-text-secondary)]">
                    Searching for your Valheim installation
                  </p>
                </>
              ) : detectError ? (
                <>
                  <XCircle
                    size={40}
                    className="mx-auto text-[var(--color-error)] mb-4"
                  />
                  <h2 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
                    Valheim Not Found
                  </h2>
                  <p className="text-sm text-[var(--color-text-secondary)] mb-6">
                    {detectError}
                  </p>
                  <button
                    onClick={handleRetryDetect}
                    className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold
                      bg-[var(--color-accent-primary)] text-white
                      hover:bg-[var(--color-accent-primary-hover)] active:scale-[0.98] transition-all cursor-pointer"
                  >
                    <Search size={16} />
                    Retry Detection
                  </button>
                </>
              ) : (
                <>
                  <CheckCircle
                    size={40}
                    className="mx-auto text-[var(--color-success)] mb-4"
                  />
                  <h2 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
                    Valheim Found!
                  </h2>
                  <p className="text-sm text-[var(--color-text-muted)] font-mono bg-[var(--color-bg-input)] px-3 py-2 rounded-md mb-6">
                    {gamePath}
                  </p>
                  <button
                    onClick={() => setStep("bepinex")}
                    className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold
                      bg-[var(--color-accent-amber)] text-white
                      hover:bg-[var(--color-accent-amber-hover)] active:scale-[0.98] transition-all cursor-pointer"
                  >
                    Continue
                    <ArrowRight size={16} />
                  </button>
                </>
              )}
            </div>
          )}

          {/* Step 2: BepInEx */}
          {step === "bepinex" && (
            <div className="text-center">
              {installing ? (
                <>
                  <Loader2
                    size={40}
                    className="mx-auto text-[var(--color-accent-primary)] animate-spin mb-4"
                  />
                  <h2 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
                    Installing BepInEx...
                  </h2>
                  <p className="text-sm text-[var(--color-text-secondary)] mb-4">
                    Setting up the mod loader framework
                  </p>
                  <ProgressBar
                    value={installProgress}
                    label="Progress"
                    className="max-w-xs mx-auto"
                  />
                </>
              ) : installError ? (
                <>
                  <XCircle
                    size={40}
                    className="mx-auto text-[var(--color-error)] mb-4"
                  />
                  <h2 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
                    Installation Failed
                  </h2>
                  <p className="text-sm text-[var(--color-text-secondary)] mb-6">
                    {installError}
                  </p>
                  <button
                    onClick={handleInstallBepinex}
                    className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold
                      bg-[var(--color-accent-primary)] text-white
                      hover:bg-[var(--color-accent-primary-hover)] active:scale-[0.98] transition-all cursor-pointer"
                  >
                    Retry Installation
                  </button>
                </>
              ) : (
                <>
                  <div className="w-12 h-12 rounded-xl bg-[var(--color-accent-primary)]/15 flex items-center justify-center mx-auto mb-4">
                    <Shield
                      size={24}
                      className="text-[var(--color-accent-primary)]"
                    />
                  </div>
                  <h2 className="text-lg font-semibold text-[var(--color-text-primary)] mb-2">
                    Install BepInEx
                  </h2>
                  <p className="text-sm text-[var(--color-text-secondary)] mb-4">
                    BepInEx is the mod loading framework required for Valheim
                    mods. It needs to be installed once.
                  </p>

                  {/* macOS Gatekeeper warning */}
                  <div className="flex items-start gap-2.5 text-left p-3 rounded-lg bg-[var(--color-warning)]/10 border border-[var(--color-warning)]/20 mb-6">
                    <AlertTriangle
                      size={16}
                      className="text-[var(--color-warning)] mt-0.5 shrink-0"
                    />
                    <p className="text-xs text-[var(--color-text-secondary)] leading-relaxed">
                      <span className="font-semibold text-[var(--color-warning)]">
                        macOS Gatekeeper:
                      </span>{" "}
                      After installation, you may need to allow BepInEx
                      libraries in System Preferences &gt; Privacy & Security if
                      prompted.
                    </p>
                  </div>

                  <button
                    onClick={handleInstallBepinex}
                    className="inline-flex items-center gap-2 px-5 py-2.5 rounded-lg text-sm font-semibold
                      bg-[var(--color-accent-primary)] text-white
                      hover:bg-[var(--color-accent-primary-hover)] active:scale-[0.98] transition-all cursor-pointer"
                  >
                    Install BepInEx
                    <ArrowRight size={16} />
                  </button>
                </>
              )}
            </div>
          )}

          {/* Step 3: Ready */}
          {step === "ready" && (
            <div className="text-center">
              <CheckCircle
                size={48}
                className="mx-auto text-[var(--color-success)] mb-4"
              />
              <h2 className="text-xl font-bold text-[var(--color-text-primary)] mb-2">
                You&apos;re All Set!
              </h2>
              <p className="text-sm text-[var(--color-text-secondary)] mb-6">
                Valheim and BepInEx are ready. Start browsing and installing
                mods.
              </p>
              <button
                onClick={handleFinish}
                className="inline-flex items-center gap-2 px-6 py-3 rounded-lg text-sm font-semibold
                  bg-gradient-to-r from-[var(--color-accent-amber)] to-orange-600 text-white
                  shadow-md shadow-orange-900/30
                  hover:from-[var(--color-accent-amber-hover)] hover:to-orange-700
                  active:scale-[0.98] transition-all cursor-pointer"
              >
                Start Managing Mods
                <ArrowRight size={16} />
              </button>
            </div>
          )}
        </div>

        {!detecting && !installing && step !== "ready" && <GameLocationPicker onSelected={status => {
          setGameStatus(status); setGamePath(status.game_path); setDetectError(null); setInstallError(null);
          setStep(status.bepinex_installed ? "ready" : "bepinex");
        }} />}

        {/* Footer */}
        <p className="text-center text-xs text-[var(--color-text-muted)] mt-6">
          Built for macOS &middot; Macheim v1.2.1
        </p>
      </div>
    </div>
  );
}
