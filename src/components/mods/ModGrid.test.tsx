import { StrictMode } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import { fetchPackages } from "../../lib/tauri";
import { useModStore } from "../../store/modStore";
import ModGrid from "./ModGrid";
vi.mock("../../lib/tauri", () => ({ fetchPackages: vi.fn() }));
beforeEach(() => { useModStore.setState({ packages: [], isLoadingPackages: false }); vi.clearAllMocks(); });
afterEach(cleanup);
test("catalog failure stays visible instead of looking like an empty search", async () => {
  vi.mocked(fetchPackages).mockRejectedValue(new Error("server unavailable"));
  render(<StrictMode><ModGrid /></StrictMode>);
  expect(await screen.findByRole("alert")).toHaveProperty("textContent", expect.stringContaining("server unavailable"));
  await waitFor(() => expect(useModStore.getState().isLoadingPackages).toBe(false));
});
test("leaving while fetching never leaves the shared refreshing flag stuck", async () => {
  let resolve!: (value: []) => void;
  vi.mocked(fetchPackages).mockReturnValue(new Promise(r => { resolve = r; }));
  const view = render(<ModGrid />);
  view.unmount(); resolve([]);
  await waitFor(() => expect(useModStore.getState().isLoadingPackages).toBe(false));
});
