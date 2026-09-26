import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { getDownloadCdn, setDownloadCdn } from "../../lib/tauri";
import DownloadSettings from "./DownloadSettings";
vi.mock("../../lib/tauri", () => ({ getDownloadCdn: vi.fn(), setDownloadCdn: vi.fn() }));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
test("loads preference without writing and saves only explicit choices", async () => {
  vi.mocked(getDownloadCdn).mockResolvedValue("automatic");
  vi.mocked(setDownloadCdn).mockResolvedValue();
  render(<DownloadSettings />);
  const select = screen.getByRole("combobox") as HTMLSelectElement;
  await waitFor(() => expect(select.disabled).toBe(false));
  expect(setDownloadCdn).not.toHaveBeenCalled();
  fireEvent.change(select, { target: { value: "hetzner" } });
  await screen.findByText("Download preference saved.");
  expect(setDownloadCdn).toHaveBeenCalledWith("hetzner");
});
test("failed persistence is visible and does not claim success", async () => {
  vi.mocked(getDownloadCdn).mockResolvedValue("automatic");
  vi.mocked(setDownloadCdn).mockRejectedValue("Permission denied");
  render(<DownloadSettings />);
  const select = screen.getByRole("combobox") as HTMLSelectElement;
  await waitFor(() => expect(select.disabled).toBe(false));
  fireEvent.change(select, { target: { value: "google" } });
  expect((await screen.findByRole("alert")).textContent).toContain("Permission denied");
  expect(select.value).toBe("automatic");
  expect(screen.queryByText("Download preference saved.")).toBeNull();
});
