import { useEffect, useState } from "react";
import { getDownloadCdn, setDownloadCdn, type DownloadCdn } from "../../lib/tauri";

export default function DownloadSettings() {
  const [cdn, setCdn] = useState<DownloadCdn>("automatic");
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    let cancelled = false;
    getDownloadCdn().then(value => { if (!cancelled) setCdn(value); })
      .catch(e => { if (!cancelled) setError(String(e)); })
      .finally(() => { if (!cancelled) setBusy(false); });
    return () => { cancelled = true; };
  }, []);
  async function save(value: DownloadCdn) {
    setBusy(true); setError(""); setSaved(false);
    try { await setDownloadCdn(value); setCdn(value); setSaved(true); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }
  return <section className="rounded-lg border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-5 space-y-3">
    <h3 className="font-semibold">Mod downloads</h3>
    <label className="text-sm flex gap-3 items-center">Thunderstore download CDN
      <select aria-label="Thunderstore download CDN" value={cdn} disabled={busy} onChange={e => void save(e.target.value as DownloadCdn)} className="bg-[var(--color-bg-input)] p-2 rounded">
        <option value="automatic">Automatic (server default)</option>
        <option value="cloudflare">Cloudflare (ccdn)</option>
        <option value="google">Google (gcdn)</option>
        <option value="hetzner">Hetzner (hcdn)</option>
      </select>
    </label>
    <p className="text-xs text-[var(--color-text-secondary)]">If a download host is blocked, you can explicitly choose another Thunderstore CDN and retry. This does not disable security software or silently fall back to another host. Hexium URLs and catalogs are unchanged. Applies to new downloads.</p>
    {error && <p role="alert">{error}</p>}
    {saved && <p role="status">Download preference saved.</p>}
  </section>;
}
