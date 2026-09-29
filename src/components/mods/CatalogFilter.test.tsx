import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { useModStore } from "../../store/modStore";
import CatalogFilter from "./CatalogFilter";
import ModCard from "./ModCard";
import { installMod, getInstalledMods } from "../../lib/tauri";
import type { ThunderstorePackage } from "../../lib/types";

vi.mock("../../lib/tauri", () => ({ installMod: vi.fn(), getInstalledMods: vi.fn() }));
afterEach(cleanup);
const pkg: ThunderstorePackage = { name: "Example", full_name: "Team-Example", owner: "Team", source: "hexium", version_number: "1.0.0", description: "", downloads: 1, rating_score: 0, is_deprecated: false, icon: "", categories: [], date_updated: "" };
test("filter keeps duplicate names distinct and selects an explicit source", () => {
  useModStore.setState({ sourceFilter: "all", searchQuery: "", packages: [pkg, {...pkg, source: "thunderstore"}] });
  render(<CatalogFilter />);
  expect(useModStore.getState().getFilteredPackages()).toHaveLength(2);
  fireEvent.change(screen.getByLabelText("Mod source"), {target: {value: "hexium"}});
  expect(useModStore.getState().getFilteredPackages()).toEqual([pkg]);
});
test("card install sends the displayed source, not the profile's legacy catalog", async () => {
  useModStore.setState({ installedMods: [], isInstallingMod: null });
  vi.mocked(installMod).mockResolvedValue([]);
  vi.mocked(getInstalledMods).mockResolvedValue([]);
  render(<ModCard pkg={pkg} />);
  fireEvent.click(screen.getByRole("button"));
  await waitFor(() => expect(installMod).toHaveBeenCalledWith(pkg.full_name, "1.0.0", "hexium"));
});
