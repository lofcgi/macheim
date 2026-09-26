import { useCallback, useEffect, useState } from "react";
import { Shield, RefreshCw, AlertTriangle } from "lucide-react";
import { getCompatibility, applyCompatibility } from "../../lib/tauri";
import type { CompatibilitySettings, CompatibilityStatus } from "../../lib/types";
import { useAppStore } from "../../store/appStore";

export default function CompatibilityPage() {
  const [status, setStatus] = useState<CompatibilityStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const addToast = useAppStore(s => s.addToast);
  const load = useCallback(async () => {
    setBusy(true); setError("");
    try { setStatus(await getCompatibility()); }
    catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }, []);
  useEffect(() => { void load(); }, [load]);
  async function apply(settings: CompatibilitySettings) {
    if (!status) return;
    setBusy(true); setError("");
    try {
      setStatus(await applyCompatibility(status.profile_name, settings));
      addToast({ type: "success", message: "Compatibility settings saved for the next game launch." });
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  }
  const button = "px-3 py-2 rounded-lg border border-[var(--color-border-default)] text-sm hover:bg-[var(--color-bg-elevated)] disabled:opacity-50 cursor-pointer";
  return <div className="max-w-3xl space-y-5">
    <section className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-5 space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div><h3 className="text-lg font-semibold flex items-center gap-2"><Shield size={20} /> Mac Compatibility</h3><p className="text-sm text-[var(--color-text-secondary)] mt-1">Version-pinned visual workarounds, controlled per profile.</p></div>
        <button className={button} onClick={load} disabled={busy}><RefreshCw size={14} className="inline mr-2" />Check support</button>
      </div>
      <p className="text-sm text-[var(--color-text-secondary)]">This checks installed mod versions and the bundled support list—not every shader in the game. Only the listed item effects are eligible. Original mod assets are not rewritten.</p>
      {error && <p role="alert" className="text-sm text-[var(--color-error)] break-words">{error}</p>}
      {!status && !error && <p role="status">Checking support…</p>}
      {status && <>
        <p className="text-sm">Profile: <strong>{status.profile_name}</strong></p>
        <div role="status" aria-label="Last recorded runtime status" className="text-sm border border-[var(--color-warning)] rounded-lg p-3">
          <p className="font-medium">Last recorded runtime status</p>
          <p className="break-words">{status.recent_log.filter(line => line.includes("[CompatibilityStatus]")).slice(-1)[0] || "Unverified: no runtime status has been recorded. Installed files do not prove the patch is active."}</p>
          <p className="text-xs mt-2">This may be from a previous session or profile. The patch only runs on the exact game and Unity versions listed below. Launch the game, then check support again.</p>
        </div>
        <label className="flex gap-3 items-center text-sm font-medium"><input type="checkbox" checked={status.settings.automatic} disabled={busy || status.game_running} onChange={e => apply({ ...status.settings, automatic: e.target.checked })} />Automatically apply verified compatibility rules</label>
        <p className="text-xs text-[var(--color-text-muted)]">Applied after mod changes and before Play Modded. Turning this off unloads Macheim's patch on the next launch; it does not disable ShaderHelper or other mods.</p>
        <div className="flex items-center justify-between gap-3"><p role="status" className="text-sm">{status.up_to_date ? (status.installed ? "Managed patch installed. Runtime checks still apply." : "No managed patch is active.") : "Installed files need to be reconciled with this profile."}</p><button className={button} disabled={busy || status.game_running} onClick={() => apply(status.settings)}>Apply supported rules</button></div>
        {status.game_running && <p className="text-sm text-[var(--color-warning)]">Quit Valheim before applying or disabling patches, then check support again.</p>}
      </>}
    </section>
    {status && <>
      <section className="rounded-xl border border-[var(--color-warning)]/30 bg-[var(--color-warning)]/5 p-4 text-sm space-y-2">
        <p className="font-medium flex items-center gap-2"><AlertTriangle size={16} /> Limited, tested coverage</p>
        <p>Runtime support: Valheim {status.catalog.game_version}, Unity {status.catalog.unity_version}, macOS Metal. The plugin skips other game/Unity versions. Mod versions come from profile metadata; manually replaced DLLs cannot be verified by that metadata.</p>
        <p>Not a universal shader repair. Backpacks, other items, monsters, buildings, equipment and UI icons are not covered. Effect brightness can differ from Windows. Automatic all-mod shader scanning is not included.</p>
      </section>
      <div className="space-y-3">{status.rules.map(({ rule, eligible, reason }) => {
        const enabled = !status.settings.disabled_rules.includes(rule.id);
        return <section key={rule.id} className="rounded-xl border border-[var(--color-border-default)] bg-[var(--color-bg-card)] p-5 space-y-2">
          <div className="flex justify-between gap-4 items-start"><div><h4 className="font-semibold">{rule.title}</h4><p className="text-xs text-[var(--color-text-muted)] mt-1">{rule.package} · {rule.version}</p></div>
            <label className="flex gap-2 text-sm items-center"><input type="checkbox" aria-label={`Enable ${rule.title}`} checked={enabled} disabled={busy || status.game_running} onChange={e => apply({ ...status.settings, disabled_rules: e.target.checked ? status.settings.disabled_rules.filter(id => id !== rule.id) : [...status.settings.disabled_rules, rule.id] })} />Allow rule</label></div>
          <p className={`text-sm ${eligible ? "text-[var(--color-success)]" : "text-[var(--color-text-secondary)]"}`}>{eligible ? "Eligible for next launch (subject to runtime version checks)." : reason}</p>
          <p className="text-sm text-[var(--color-text-secondary)]">{rule.reason}</p>
          <details className="text-xs text-[var(--color-text-muted)]"><summary className="cursor-pointer">Tested objects and limitations</summary><p className="mt-2 break-words">{rule.prefabs.join(", ")}</p><p className="mt-2">{rule.validation}</p></details>
        </section>;
      })}</div>
      <details className="rounded-xl border border-[var(--color-border-default)] p-4 text-sm"><summary className="cursor-pointer">Recent compatibility log (local only)</summary><p className="text-xs text-[var(--color-text-muted)] mt-2">From the game's latest log; may describe a previous profile or session. An empty log does not mean every shader passed. No logs are uploaded.</p><pre className="mt-3 whitespace-pre-wrap break-all text-xs max-h-64 overflow-y-auto">{status.recent_log.join("\n") || "No recent Macheim compatibility entries. Launch the game and encounter a supported item, then check again."}</pre></details>
    </>}
  </div>;
}
