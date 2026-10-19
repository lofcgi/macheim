import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { check, type DownloadEvent, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { getVersion } from "@tauri-apps/api/app";
import { useUpdateStore } from "../../store/updateStore";
import UpdateBanner from "./UpdateBanner";
import AppUpdateSettings from "../settings/AppUpdateSettings";
vi.mock("@tauri-apps/plugin-updater", () => ({ check: vi.fn() }));
vi.mock("@tauri-apps/plugin-process", () => ({ relaunch: vi.fn() }));
vi.mock("@tauri-apps/api/app", () => ({ getVersion: vi.fn() }));
const initial = useUpdateStore.getState();
beforeEach(() => { vi.mocked(getVersion).mockResolvedValue("1.4.0"); });
afterEach(() => { cleanup(); vi.clearAllMocks(); useUpdateStore.setState(initial, true); });
function fakeUpdate(downloadAndInstall: Update["downloadAndInstall"]) {
  return { version: "1.4.1", currentVersion: "1.4.0", downloadAndInstall } as Update;
}
test("shows nothing when the app is current, and says so in settings", async () => {
  vi.mocked(check).mockResolvedValue(null);
  render(<><UpdateBanner /><AppUpdateSettings /></>);
  await act(() => useUpdateStore.getState().checkForUpdate());
  expect(screen.queryByRole("region", { name: "App update" })).toBeNull();
  expect(screen.getByRole("status").textContent).toBe("Macheim is up to date.");
  expect(await screen.findByText("Macheim v1.4.0")).toBeTruthy();
});
test("installs the signed update with progress, then relaunches", async () => {
  let finish!: () => void;
  const downloadAndInstall = vi.fn((onEvent?: (e: DownloadEvent) => void) => new Promise<void>(resolve => {
    onEvent?.({ event: "Started", data: { contentLength: 200 } });
    onEvent?.({ event: "Progress", data: { chunkLength: 50 } });
    finish = () => { onEvent?.({ event: "Finished" }); resolve(); };
  }));
  vi.mocked(check).mockResolvedValue(fakeUpdate(downloadAndInstall));
  vi.mocked(relaunch).mockResolvedValue();
  render(<UpdateBanner />);
  await act(() => useUpdateStore.getState().checkForUpdate());
  const banner = screen.getByRole("region", { name: "App update" });
  expect(banner.textContent).toContain("Macheim 1.4.1 is available. You have 1.4.0.");
  expect(screen.getByRole("link", { name: "Release notes" }).getAttribute("href")).toBe("https://github.com/lofcgi/macheim/releases/tag/v1.4.1");
  fireEvent.click(screen.getByRole("button", { name: "Install and restart" }));
  await screen.findByText("Downloading Macheim 1.4.1… 25%");
  expect((screen.getByRole("button", { name: "Installing..." }) as HTMLButtonElement).disabled).toBe(true);
  expect(relaunch).not.toHaveBeenCalled();
  await act(async () => finish());
  await waitFor(() => expect(relaunch).toHaveBeenCalledTimes(1));
});
test("a failed install is visible, does not relaunch, and can be retried", async () => {
  const downloadAndInstall = vi.fn().mockRejectedValue("signature verification failed");
  vi.mocked(check).mockResolvedValue(fakeUpdate(downloadAndInstall));
  render(<UpdateBanner />);
  await act(() => useUpdateStore.getState().checkForUpdate());
  fireEvent.click(screen.getByRole("button", { name: "Install and restart" }));
  expect((await screen.findByRole("alert")).textContent).toContain("Update failed: signature verification failed");
  expect(relaunch).not.toHaveBeenCalled();
  expect((screen.getByRole("button", { name: "Install and restart" }) as HTMLButtonElement).disabled).toBe(false);
});
test("a failed check is reported in settings instead of being dropped", async () => {
  vi.mocked(check).mockRejectedValue("Could not fetch a valid release JSON from the remote");
  render(<><UpdateBanner /><AppUpdateSettings /></>);
  await act(() => useUpdateStore.getState().checkForUpdate());
  expect(screen.queryByRole("region", { name: "App update" })).toBeNull();
  expect(screen.getByRole("alert").textContent).toContain("Could not fetch a valid release JSON");
  vi.mocked(check).mockResolvedValue(null);
  fireEvent.click(screen.getByRole("button", { name: "Check for updates" }));
  await screen.findByText("Macheim is up to date.");
  expect(check).toHaveBeenCalledTimes(2);
});
test("dismissing hides the banner but settings still offers the update", async () => {
  vi.mocked(check).mockResolvedValue(fakeUpdate(vi.fn()));
  render(<><UpdateBanner /><AppUpdateSettings /></>);
  await act(() => useUpdateStore.getState().checkForUpdate());
  fireEvent.click(screen.getByRole("button", { name: "Dismiss update" }));
  expect(screen.queryByRole("region", { name: "App update" })).toBeNull();
  expect(screen.getByRole("button", { name: "Install 1.4.1 and restart" })).toBeTruthy();
});
