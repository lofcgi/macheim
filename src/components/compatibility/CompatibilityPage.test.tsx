import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { getCompatibility, applyCompatibility } from "../../lib/tauri";
import type { CompatibilityStatus } from "../../lib/types";
import CompatibilityPage from "./CompatibilityPage";
vi.mock("../../lib/tauri", () => ({ getCompatibility: vi.fn(), applyCompatibility: vi.fn() }));
const status: CompatibilityStatus = {
  profile_name: "Friends", settings: { automatic: true, disabled_rules: [] },
  catalog: { revision: 1, plugin_version: "0.2.0", game_version: "0.221.12", unity_version: "6000.0.61f1", rules: [], requirements: [] },
  rules: [], installed: false, up_to_date: true, game_running: false, recent_log: [],
};
beforeEach(() => { vi.clearAllMocks(); vi.mocked(getCompatibility).mockResolvedValue(structuredClone(status)); });
afterEach(cleanup);
test("runtime rejection is visible outside the collapsed log", async () => {
  vi.mocked(getCompatibility).mockResolvedValue({ ...status, recent_log: ["[CompatibilityStatus] inactive: unverified game/Unity version 1.0.14/6000.0"] });
  render(<CompatibilityPage />);
  expect((await screen.findByRole("status", { name: "Last recorded runtime status" })).textContent).toContain("inactive: unverified");
});
test("checking support is read-only and never claims a full shader scan", async () => {
  render(<CompatibilityPage />); await screen.findByText("Friends");
  expect(screen.getByText(/not every shader in the game/)).toBeTruthy();
  expect(applyCompatibility).not.toHaveBeenCalled();
});
test("automatic opt-out is scoped to the shown profile", async () => {
  vi.mocked(applyCompatibility).mockResolvedValue({ ...status, settings: { automatic: false, disabled_rules: [] } });
  render(<CompatibilityPage />); await screen.findByText("Friends");
  fireEvent.click(screen.getByRole("checkbox", { name: "Automatically apply verified compatibility rules" }));
  await waitFor(() => expect(applyCompatibility).toHaveBeenCalledWith("Friends", { automatic: false, disabled_rules: [] }));
});
test("game-running state blocks apply and disable controls", async () => {
  vi.mocked(getCompatibility).mockResolvedValue({ ...status, game_running: true });
  render(<CompatibilityPage />); await screen.findByText("Friends");
  expect((screen.getByText("Apply supported rules") as HTMLButtonElement).disabled).toBe(true);
  expect((screen.getByRole("checkbox") as HTMLInputElement).disabled).toBe(true);
});
test("backend errors remain visible without a blank screen", async () => {
  vi.mocked(getCompatibility).mockRejectedValue("Profile missing");
  render(<CompatibilityPage />);
  expect((await screen.findByRole("alert")).textContent).toContain("Profile missing");
  expect(screen.getByText("Check support")).toBeTruthy();
});
