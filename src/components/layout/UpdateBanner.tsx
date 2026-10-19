import { Download, ExternalLink, X } from "lucide-react";
import { formatProgress, releaseUrl, useUpdateStore } from "../../store/updateStore";

export default function UpdateBanner() {
  const update = useUpdateStore((s) => s.update);
  const status = useUpdateStore((s) => s.status);
  const error = useUpdateStore((s) => s.error);
  const downloaded = useUpdateStore((s) => s.downloaded);
  const total = useUpdateStore((s) => s.total);
  const dismissed = useUpdateStore((s) => s.dismissed);
  const installUpdate = useUpdateStore((s) => s.installUpdate);
  const dismiss = useUpdateStore((s) => s.dismiss);

  if (!update || dismissed) return null;
  const installing = status === "installing";

  return (
    <div
      role="region"
      aria-label="App update"
      className="shrink-0 flex flex-wrap items-center gap-x-4 gap-y-1 px-6 py-2 text-sm border-b border-[var(--color-border-subtle)] bg-[var(--color-accent-primary)]/10"
    >
      <Download size={16} className="shrink-0 text-[var(--color-accent-primary)]" />
      <p className="flex-1 min-w-0 text-[var(--color-text-primary)]">
        {installing
          ? `Downloading Macheim ${update.version}… ${formatProgress(downloaded, total)}`
          : `Macheim ${update.version} is available. You have ${update.currentVersion}.`}
      </p>
      <a
        href={releaseUrl(update.version)}
        target="_blank"
        rel="noopener noreferrer"
        className="inline-flex items-center gap-1.5 text-[var(--color-accent-primary)] hover:text-[var(--color-accent-primary-hover)] transition-colors"
      >
        <ExternalLink size={14} />
        Release notes
      </a>
      <button
        onClick={() => void installUpdate()}
        disabled={installing}
        className="text-xs px-3 py-1.5 rounded-md bg-[var(--color-accent-primary)] text-white
          hover:bg-[var(--color-accent-primary-hover)] disabled:opacity-50 transition-colors cursor-pointer"
      >
        {installing ? "Installing..." : "Install and restart"}
      </button>
      {!installing && (
        <button
          onClick={dismiss}
          aria-label="Dismiss update"
          className="p-1 rounded hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
        >
          <X size={14} className="text-[var(--color-text-muted)]" />
        </button>
      )}
      {status === "error" && (
        <p role="alert" className="basis-full text-xs text-[var(--color-error)]">
          Update failed: {error}. You can also download it from the release notes page.
        </p>
      )}
    </div>
  );
}
