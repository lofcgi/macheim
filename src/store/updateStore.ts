import { create } from "zustand";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "installing"
  | "error";

export function releaseUrl(version: string): string {
  return `https://github.com/lofcgi/macheim/releases/tag/v${version}`;
}

export function formatProgress(downloaded: number, total: number | null): string {
  if (total) return `${Math.min(100, Math.round((downloaded / total) * 100))}%`;
  return `${(downloaded / 1_000_000).toFixed(1)} MB`;
}

interface UpdateState {
  status: UpdateStatus;
  update: Update | null;
  error: string;
  downloaded: number;
  total: number | null;
  dismissed: boolean;

  checkForUpdate: () => Promise<void>;
  installUpdate: () => Promise<void>;
  dismiss: () => void;
}

export const useUpdateStore = create<UpdateState>((set, get) => ({
  status: "idle",
  update: null,
  error: "",
  downloaded: 0,
  total: null,
  dismissed: false,

  checkForUpdate: async () => {
    const { status } = get();
    if (status === "checking" || status === "installing") return;
    set({ status: "checking", error: "" });
    try {
      const update = await check();
      set({ update, status: update ? "available" : "current", dismissed: false });
    } catch (e) {
      set({ status: "error", error: String(e) });
    }
  },

  installUpdate: async () => {
    const { update, status } = get();
    if (!update || status === "installing") return;
    set({ status: "installing", error: "", downloaded: 0, total: null });
    try {
      await update.downloadAndInstall((event) => {
        if (event.event === "Started") {
          set({ total: event.data.contentLength ?? null });
        } else if (event.event === "Progress") {
          set((s) => ({ downloaded: s.downloaded + event.data.chunkLength }));
        }
      });
      await relaunch();
    } catch (e) {
      set({ status: "error", error: String(e) });
    }
  },

  dismiss: () => set({ dismissed: true }),
}));
