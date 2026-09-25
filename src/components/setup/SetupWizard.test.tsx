import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { detectGame } from "../../lib/tauri";
import { invoke } from "@tauri-apps/api/core";
import SetupWizard from "./SetupWizard";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("../../lib/tauri", async (original) => ({
  ...await original<typeof import("../../lib/tauri")>(), detectGame: vi.fn(),
}));
afterEach(() => { cleanup(); vi.clearAllMocks(); });
test("failed detection offers a manual path and advances after validation", async () => {
  vi.mocked(detectGame).mockRejectedValue("Not found");
  vi.mocked(invoke).mockResolvedValue({ installed: true, game_path: "/Volumes/Games/Valheim/valheim.app", bepinex_installed: false, active_profile: "Default" });
  render(<SetupWizard />);
  await screen.findByText(/Could not detect Valheim/);
  fireEvent.change(screen.getByLabelText("Valheim location"), { target: { value: "/Volumes/Games/Valheim" } });
  fireEvent.click(screen.getByRole("button", { name: "Use this location" }));
  await waitFor(() => expect(invoke).toHaveBeenCalledWith("set_game_path", { path: "/Volumes/Games/Valheim" }));
  expect(await screen.findByRole("button", { name: /Install BepInEx/ })).toBeTruthy();
});
