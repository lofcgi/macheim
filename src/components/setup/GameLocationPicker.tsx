import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { setGamePath } from "../../lib/tauri";
import type { GameStatus } from "../../lib/types";

export default function GameLocationPicker({ onSelected }: { onSelected: (status: GameStatus) => void }) {
  const [path, setPath] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  async function select(value: string) {
    setBusy(true); setError(null);
    try { onSelected(await setGamePath(value.trim())); }
    catch (err) { setError(String(err)); }
    finally { setBusy(false); }
  }
  return <div className="mt-4 space-y-2 text-left text-sm">
    <label htmlFor="game-location">Valheim location</label>
    <input id="game-location" value={path} onChange={e => setPath(e.target.value)}
      placeholder="/Volumes/Games/steamapps/common/Valheim"
      className="w-full rounded border border-[var(--color-border-default)] bg-[var(--color-bg-input)] p-2" />
    <p className="text-xs text-[var(--color-text-muted)]">Choose the folder containing valheim.app, or paste the app path. External drives are supported. Close Valheim before changing locations.</p>
    <div className="flex gap-4">
      <button disabled={busy || !path.trim()} onClick={() => void select(path)} className="underline disabled:opacity-40">{busy ? "Checking location..." : "Use this location"}</button>
      <button disabled={busy} className="underline" onClick={async () => {
        setError(null);
        try {
          const value = await open({ directory: true, multiple: false, title: "Select the folder containing valheim.app" });
          if (typeof value === "string") { setPath(value); await select(value); }
        } catch (err) { setError(String(err)); }
      }}>Choose folder…</button>
    </div>
    {error && <p role="alert" className="text-[var(--color-error)]">{error}</p>}
  </div>;
}
