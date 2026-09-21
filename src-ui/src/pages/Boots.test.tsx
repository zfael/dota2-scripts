import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { fireEvent, render, screen } from "@testing-library/react";
import Boots from "./Boots";
import { useConfigStore } from "../stores/configStore";
import { mockConfig } from "../stores/mockData";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("../lib/tauri", () => ({
  isTauri: () => true,
}));

function setPhaseBoots() {
  useConfigStore.setState({
    config: {
      ...mockConfig,
      phase_boots_automation: {
        enabled: true,
        minimum_distance_units: 100,
        // Not editable from the UI; configured through config.toml only.
        excluded_heroes: [],
      },
    },
    loaded: true,
  });
}

describe("Boots page phase boots automation controls", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useFakeTimers();
    setPhaseBoots();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("renders phase boots movement controls on the Boots page", () => {
    render(
      <MemoryRouter>
        <Boots />
      </MemoryRouter>,
    );

    expect(screen.getByText("Boots")).toBeInTheDocument();
    expect(screen.getByText("Phase Boots")).toBeInTheDocument();
    expect(screen.getByText("Enable Phase Boots Automation")).toBeInTheDocument();
    expect(screen.getByText("Minimum Movement Distance")).toBeInTheDocument();
    expect(screen.queryByText("Excluded Heroes")).not.toBeInTheDocument();
  });

  // The switch moved to Survivability once it stopped being about Phase Boots.
  it("points at Survivability for the invisibility hold instead of owning it", () => {
    render(
      <MemoryRouter>
        <Boots />
      </MemoryRouter>,
    );

    expect(
      screen.queryByRole("switch", { name: "Hold While Invisible" }),
    ).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Survivability" })).toHaveAttribute(
      "href",
      "/survivability",
    );
  });

  it("persists phase boots toggle changes through the shared config store", async () => {
    invokeMock.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <Boots />
      </MemoryRouter>,
    );

    await fireEvent.click(
      screen.getByRole("switch", { name: "Enable Phase Boots Automation" }),
    );

    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "phase_boots_automation",
        updates: {
          enabled: false,
        },
      });
    });
  });

  it("ships Power Treads off, with its behaviour toggles disabled until enabled", () => {
    render(
      <MemoryRouter>
        <Boots />
      </MemoryRouter>,
    );

    expect(screen.getByText("Power Treads")).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "Enable Power Treads Switching" }),
    ).toHaveAttribute("aria-checked", "false");
    for (const name of [
      "Switch for Mana Spells and Items",
      "Hold STR in Danger",
      "Heal in the Smallest Pool",
      "Verify State When Idle",
      "My Switches Set Home",
    ]) {
      expect(screen.getByRole("switch", { name })).toBeDisabled();
    }
  });

  it("persists the Power Treads master switch to its own section", async () => {
    invokeMock.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <Boots />
      </MemoryRouter>,
    );

    await fireEvent.click(
      screen.getByRole("switch", { name: "Enable Power Treads Switching" }),
    );

    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "power_treads",
        updates: {
          enabled: true,
        },
      });
    });
  });

});
