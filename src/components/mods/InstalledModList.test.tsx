import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { getInstalledMods, listUnmanagedMods, syncMods, checkModUpdates, updateMods } from "../../lib/tauri";
import { useModStore } from "../../store/modStore";
import InstalledModList from "./InstalledModList";
vi.mock("../../lib/tauri", () => ({ getInstalledMods: vi.fn(), listUnmanagedMods: vi.fn(), syncMods: vi.fn(), toggleMod: vi.fn(), uninstallMod: vi.fn(), checkModUpdates: vi.fn(), changeModVersion: vi.fn(), updateMods: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: vi.fn() }));
const mod = { full_name: "Therzie-Wizardry", author: "Therzie", name: "Wizardry", version: "1.1.8", enabled: true, description: "", icon: "", dependencies: [], installed_at: "" };
beforeEach(() => {
  vi.clearAllMocks(); useModStore.setState({ installedMods: [], isLoadingInstalled: false });
  vi.mocked(getInstalledMods).mockResolvedValue([mod]);
  vi.mocked(listUnmanagedMods).mockResolvedValue(["Manual-Mod"]);
  vi.mocked(syncMods).mockResolvedValue({ cleaned: [], failed: [], reinstalled: [] });
});
afterEach(cleanup);
test("batch updates require confirmation and use one profile-scoped transaction", async () => {
  vi.mocked(checkModUpdates).mockResolvedValue([{ full_name: mod.full_name, name: mod.name, current_version: "1.1.8", latest_version: "1.1.9" }]);
  vi.mocked(updateMods).mockResolvedValue([{ ...mod, version: "1.1.9" }]);
  vi.mocked(confirm).mockResolvedValue(false);
  render(<InstalledModList />); await screen.findByText("Wizardry");
  fireEvent.click(screen.getByText("Check updates"));
  fireEvent.click(await screen.findByText("Update all (1)"));
  await waitFor(() => expect(confirm).toHaveBeenCalled());
  expect(updateMods).not.toHaveBeenCalled();
  vi.mocked(confirm).mockResolvedValue(true);
  fireEvent.click(screen.getByText("Update all (1)"));
  await waitFor(() => expect(updateMods).toHaveBeenCalledWith([[mod.full_name, "1.1.9"]], "Default"));
});
test("search reads the Rust author field and does not blank", async () => {
  render(<InstalledModList />);
  await screen.findByText("by Therzie");
  fireEvent.change(screen.getByPlaceholderText("Search installed mods..."), { target: { value: "Therzie" } });
  expect(screen.getByText("Wizardry")).toBeTruthy();
  fireEvent.change(screen.getByPlaceholderText("Search installed mods..."), { target: { value: "does-not-exist" } });
  expect(screen.queryByText("Wizardry")).toBeNull();
});
test("canceling native cleanup confirmation makes no mutation", async () => {
  vi.mocked(confirm).mockResolvedValue(false);
  render(<InstalledModList />); await screen.findByText("Wizardry");
  fireEvent.click(screen.getByText("Sync & Clean"));
  await waitFor(() => expect(confirm).toHaveBeenCalled());
  await screen.findByText("Sync & Clean");
  expect(syncMods).not.toHaveBeenCalled();
});
test("awaits confirmation before sending approved names", async () => {
  let answer!: (value: boolean) => void;
  vi.mocked(confirm).mockReturnValue(new Promise(resolve => { answer = resolve; }));
  render(<InstalledModList />); await screen.findByText("Wizardry");
  fireEvent.click(screen.getByText("Sync & Clean"));
  await waitFor(() => expect(confirm).toHaveBeenCalled());
  expect(syncMods).not.toHaveBeenCalled();
  answer(true);
  await waitFor(() => expect(syncMods).toHaveBeenCalledWith(true, ["Manual-Mod"]));
});
