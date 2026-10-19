import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { formatProgress, useUpdateStore } from "../../store/updateStore";

export default function AppUpdateSettings() {
  const [version, setVersion] = useState("");
  const status = useUpdateStore((s) => s.status);
  const update = useUpdateStore((s) => s.update);
  const error = useUpdateStore((s) => s.error);
  const downloaded = useUpdateStore((s) => s.downloaded);
  const total = useUpdateStore((s) => s.total);
  const checkForUpdate = useUpdateStore((s) => s.checkForUpdate);
  const installUpdate = useUpdateStore((s) => s.installUpdate);
  useEffect(() => {
    getVersion().then(v => setVersion(`v${v}`)).catch(e => setVersion(`(version unavailable: ${e})`));
  }, []);
  const busy = status === "checking" || status === "installing";
  return <section className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-5 space-y-3">
    <h3 className="font-semibold">App updates</h3>
    <div className="text-sm flex gap-3 items-center">
      <span className="text-[var(--color-text-secondary)]">Macheim {version}</span>
      <div className="flex-1" />
      {update && status !== "checking" && <button onClick={() => void installUpdate()} disabled={busy} className="text-xs px-3 py-1.5 rounded-md bg-[var(--color-accent-primary)] text-white hover:bg-[var(--color-accent-primary-hover)] disabled:opacity-50 transition-colors cursor-pointer">
        {status === "installing" ? "Installing..." : `Install ${update.version} and restart`}
      </button>}
      <button onClick={() => void checkForUpdate()} disabled={busy} className="text-xs px-3 py-1.5 rounded-md border border-[var(--color-border-default)] text-[var(--color-text-secondary)] hover:bg-[var(--color-bg-elevated)] disabled:opacity-50 transition-colors cursor-pointer">
        {status === "checking" ? "Checking..." : "Check for updates"}
      </button>
    </div>
    {status === "current" && <p role="status" className="text-sm">Macheim is up to date.</p>}
    {status === "available" && update && <p role="status" className="text-sm">Macheim {update.version} is available.</p>}
    {status === "installing" && <p role="status" className="text-sm">Downloading the update… {formatProgress(downloaded, total)}</p>}
    {status === "error" && <p role="alert" className="text-sm text-[var(--color-error)]">{error}</p>}
    <p className="text-xs text-[var(--color-text-secondary)]">Macheim checks GitHub Releases for a new version when it starts. Updates are verified with Macheim's update signing key before they replace the app, then Macheim restarts. Your profiles, mods and settings are kept.</p>
  </section>;
}
