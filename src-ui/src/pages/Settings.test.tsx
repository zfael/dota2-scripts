import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { fireEvent, render, screen } from "@testing-library/react";
import Settings from "./Settings";
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

describe("Settings page phase boots automation controls", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useFakeTimers();

    useConfigStore.setState({
      config: {
        ...mockConfig,
        phase_boots_automation: {
          enabled: true,
          minimum_distance_units: 100,
          excluded_heroes: [],
          suppress_while_invisible: true,
        },
      },
      loaded: true,
    });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("renders phase boots movement controls on the Settings page", () => {
    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>,
    );

    expect(screen.getByText("Phase Boots Automation")).toBeInTheDocument();
    expect(screen.getByText("Enable Phase Boots Automation")).toBeInTheDocument();
    expect(screen.getByText("Minimum Movement Distance")).toBeInTheDocument();
    expect(screen.getByText("Hold While Invisible")).toBeInTheDocument();
  });

  it("persists phase boots toggle changes through the shared config store", async () => {
    invokeMock.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <Settings />
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

  it("persists the invisibility hold toggle through the shared config store", async () => {
    invokeMock.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>,
    );

    await fireEvent.click(
      screen.getByRole("switch", { name: "Hold While Invisible" }),
    );

    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "phase_boots_automation",
        updates: {
          suppress_while_invisible: false,
        },
      });
    });
  });
});
