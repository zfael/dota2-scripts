import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { fireEvent, render, screen } from "@testing-library/react";
import Armlet from "./Armlet";
import { useConfigStore } from "../stores/configStore";
import { useUIStore } from "../stores/uiStore";
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

describe("Armlet page Roshan controls", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useFakeTimers();

    useConfigStore.setState({
      config: {
        ...mockConfig,
        armlet: {
          ...mockConfig.armlet,
          roshan: {
            enabled: false,
            toggle_key: "Insert",
            emergency_margin_hp: 60,
            learning_window_ms: 5000,
            min_confidence_hits: 2,
            min_sample_damage: 80,
            stale_reset_ms: 6000,
          },
        },
      },
      loaded: true,
    });

    useUIStore.setState({
      armletRoshanArmed: true,
      gsiEnabled: true,
      standaloneEnabled: false,
      appVersion: "0.15.0",
    });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("renders Roshan controls and current live mode status", () => {
    render(
      <MemoryRouter>
        <Armlet />
      </MemoryRouter>,
    );

    expect(screen.getByText("Roshan Mode")).toBeInTheDocument();
    expect(screen.getByText("Roshan Toggle Key")).toBeInTheDocument();
    expect(screen.getByText("Current Status")).toBeInTheDocument();
    expect(screen.getByText("Armed")).toBeInTheDocument();
  });

  it("persists Roshan key changes through the shared armlet config section", async () => {
    invokeMock.mockResolvedValue(undefined);

    render(
      <MemoryRouter>
        <Armlet />
      </MemoryRouter>,
    );

    const keyButton = screen.getByRole("button", { name: "Insert" });
    fireEvent.click(keyButton);
    fireEvent.keyDown(keyButton, { key: "Delete" });
    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "armlet",
        updates: {
          roshan: expect.objectContaining({
            toggle_key: "Delete",
            enabled: false,
          }),
        },
      });
    });
  });
});
