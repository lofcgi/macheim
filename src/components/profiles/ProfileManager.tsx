import { useEffect, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import {
  Plus,
  Trash2,
  User,
  Check,
  X,
  Clock,
  Package,
} from "lucide-react";
import { useProfileStore } from "../../store/profileStore";
import { useAppStore } from "../../store/appStore";
import {
  listProfiles,
  createProfile,
  switchProfile,
  deleteProfile,
} from "../../lib/tauri";

function formatDate(dateStr: string): string {
  try {
    return new Date(dateStr).toLocaleDateString("en-US", {
      year: "numeric",
      month: "short",
      day: "numeric",
    });
  } catch {
    return dateStr;
  }
}

export default function ProfileManager() {
  const profiles = useProfileStore((s) => s.profiles);
  const activeProfile = useProfileStore((s) => s.activeProfile);
  const setProfiles = useProfileStore((s) => s.setProfiles);
  const setActiveProfile = useProfileStore((s) => s.setActiveProfile);
  const addToast = useAppStore((s) => s.addToast);

  const [isCreating, setIsCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [catalogSource, setCatalogSource] = useState("thunderstore");
  const [deletingProfile, setDeletingProfile] = useState<string | null>(null);

  useEffect(() => {
    async function load() {
      try {
        const data = await listProfiles();
        setProfiles(data);
      } catch {
        // ok
      }
    }
    load();
  }, [setProfiles]);

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name) return;

    try {
      const profile = await createProfile(name, catalogSource);
      setProfiles([...profiles, profile]);
      setNewName("");
      setIsCreating(false);
      addToast({ type: "success", message: `Created profile "${name}"` });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to create profile: ${err}`,
      });
    }
  };

  const handleSwitch = async (name: string) => {
    if (name === activeProfile) return;
    try {
      await switchProfile(name);
      setActiveProfile(name);
      addToast({ type: "success", message: `Switched to "${name}"` });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to switch profile: ${err}`,
      });
    }
  };

  const handleDelete = async (name: string) => {
    if (!await confirm(`Remove profile "${name}"? Its files will be preserved in Macheim's deleted-profiles folder.`, { title: "Remove profile", kind: "warning" })) return;
    if (name === activeProfile) {
      addToast({
        type: "warning",
        message: "Cannot delete the active profile. Switch to another first.",
      });
      return;
    }

    setDeletingProfile(name);
    try {
      await deleteProfile(name);
      setProfiles(profiles.filter((p) => p.name !== name));
      addToast({ type: "info", message: `Removed "${name}". Recoverable from the deleted-profiles data folder.` });
    } catch (err) {
      addToast({
        type: "error",
        message: `Failed to delete profile: ${err}`,
      });
    } finally {
      setDeletingProfile(null);
    }
  };

  return (
    <div className="max-w-2xl">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <h3 className="text-base font-semibold text-[var(--color-text-primary)]">
            Mod Profiles
          </h3>
          <p className="text-sm text-[var(--color-text-muted)] mt-0.5">
            Manage separate mod configurations for different playstyles.
          </p>
        </div>
        <button
          onClick={() => setIsCreating(true)}
          className="inline-flex items-center gap-1.5 px-3.5 py-2 rounded-lg text-sm font-medium
            bg-[var(--color-accent-primary)] text-white
            hover:bg-[var(--color-accent-primary-hover)] active:scale-[0.98] transition-all cursor-pointer"
        >
          <Plus size={16} />
          New Profile
        </button>
      </div>

      {/* Create form */}
      {isCreating && (
        <div className="flex items-center gap-2 mb-4 p-3 rounded-lg border border-[var(--color-accent-primary)]/30 bg-[var(--color-accent-primary)]/5">
          <select aria-label="Mod catalog" value={catalogSource} onChange={e => setCatalogSource(e.target.value)} className="bg-[var(--color-bg-input)] p-2 rounded">
            <option value="thunderstore">Thunderstore</option>
            <option value="hexium">Hexium</option>
          </select>
          <input
            type="text"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleCreate();
              if (e.key === "Escape") {
                setIsCreating(false);
                setNewName("");
              }
            }}
            placeholder="Profile name..."
            autoFocus
            className="flex-1 px-3 py-1.5 rounded-md text-sm
              bg-[var(--color-bg-input)] border border-[var(--color-border-default)]
              text-[var(--color-text-primary)] placeholder:text-[var(--color-text-muted)]
              focus:outline-none focus:border-[var(--color-accent-primary)]"
          />
          <button
            onClick={handleCreate}
            disabled={!newName.trim()}
            className="p-2 rounded-md bg-[var(--color-accent-primary)] text-white
              hover:bg-[var(--color-accent-primary-hover)] disabled:opacity-50 transition-colors cursor-pointer"
          >
            <Check size={16} />
          </button>
          <button
            onClick={() => {
              setIsCreating(false);
              setNewName("");
            }}
            className="p-2 rounded-md text-[var(--color-text-muted)] hover:bg-[var(--color-bg-card)] transition-colors cursor-pointer"
          >
            <X size={16} />
          </button>
        </div>
      )}

      {/* Profile list */}
      <div className="space-y-2">
        {profiles.map((profile) => {
          const isActive = profile.name === activeProfile;
          const isDeleting = deletingProfile === profile.name;

          return (
            <div
              key={profile.name}
              className={`flex items-center gap-4 p-4 rounded-lg border transition-all
                ${
                  isActive
                    ? "border-[var(--color-accent-primary)]/30 bg-[var(--color-accent-primary)]/5"
                    : "border-[var(--color-border-subtle)] bg-[var(--color-bg-card)] hover:bg-[var(--color-bg-card-hover)]"
                }
              `}
            >
              <div
                className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0
                  ${
                    isActive
                      ? "bg-[var(--color-accent-primary)]/15"
                      : "bg-[var(--color-bg-input)]"
                  }
                `}
              >
                <User
                  size={18}
                  className={
                    isActive
                      ? "text-[var(--color-accent-primary)]"
                      : "text-[var(--color-text-muted)]"
                  }
                />
              </div>

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <h4 className="text-sm font-semibold text-[var(--color-text-primary)]">
                    {profile.name}
                  </h4>
                  <span className="text-xs text-[var(--color-text-muted)]">{profile.catalog_source === "hexium" ? "Hexium" : "Thunderstore"}</span>
                  {isActive && (
                    <span className="text-[10px] font-semibold uppercase tracking-wide px-1.5 py-0.5 rounded-full bg-[var(--color-accent-primary)]/20 text-[var(--color-accent-primary)]">
                      Active
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-3 mt-0.5 text-xs text-[var(--color-text-muted)]">
                  <span className="flex items-center gap-1">
                    <Package size={11} />
                    {profile.mods.length} mods
                  </span>
                  <span className="flex items-center gap-1">
                    <Clock size={11} />
                    Updated {formatDate(profile.updated_at)}
                  </span>
                </div>
              </div>

              {!isActive && (
                <button
                  onClick={() => handleSwitch(profile.name)}
                  className="px-3 py-1.5 rounded-md text-xs font-medium
                    border border-[var(--color-border-default)] text-[var(--color-text-secondary)]
                    hover:bg-[var(--color-bg-elevated)] hover:text-[var(--color-text-primary)]
                    transition-colors cursor-pointer"
                >
                  Switch
                </button>
              )}

              {!isActive && (
                <button
                  onClick={() => handleDelete(profile.name)}
                  disabled={isDeleting}
                  className="p-2 rounded-lg text-[var(--color-text-muted)] hover:text-[var(--color-error)] hover:bg-[var(--color-error)]/10
                    transition-colors disabled:opacity-50 cursor-pointer"
                  title="Delete profile"
                >
                  <Trash2 size={16} />
                </button>
              )}
            </div>
          );
        })}
      </div>

      {profiles.length === 0 && (
        <div className="flex flex-col items-center justify-center py-16 text-center">
          <User
            size={48}
            className="text-[var(--color-text-muted)] mb-4"
          />
          <h3 className="text-lg font-semibold text-[var(--color-text-secondary)] mb-1">
            No profiles
          </h3>
          <p className="text-sm text-[var(--color-text-muted)]">
            Create a profile to get started.
          </p>
        </div>
      )}
    </div>
  );
}
