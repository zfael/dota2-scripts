import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { fireEvent, render, screen } from "@testing-library/react";
import Survivability from "./Survivability";
import DangerDetection from "./DangerDetection";
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

function renderSurvivability() {
  return render(
    <MemoryRouter>
      <Survivability />
    </MemoryRouter>,
  );
}

describe("Survivability page", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    vi.useFakeTimers();
    useConfigStore.setState({
      config: JSON.parse(JSON.stringify(mockConfig)),
      loaded: true,
    });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("gathers every survivability response on one page", () => {
    renderSurvivability();

    expect(screen.getByText("Healing Items")).toBeInTheDocument();
    expect(screen.getByText("Lane Phase")).toBeInTheDocument();
    expect(screen.getByText("Defensive Items")).toBeInTheDocument();
    expect(screen.getByText("Dispels")).toBeInTheDocument();
    expect(screen.getByText("Neutral Items")).toBeInTheDocument();
    expect(screen.getByText("Invisibility")).toBeInTheDocument();
  });

  it("persists the invisibility hold to its own section", async () => {
    renderSurvivability();

    fireEvent.click(
      screen.getByRole("switch", { name: "Hold Automation While Invisible" }),
    );

    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "invisibility",
        updates: { suppress_automation: false },
      });
    });
  });

  it("persists the shared healing threshold to the common section", async () => {
    renderSurvivability();

    fireEvent.change(
      screen.getByRole("slider", { name: "Healing HP Threshold" }),
      { target: { value: "45" } },
    );

    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "common",
        updates: { survivability_hp_threshold: 45 },
      });
    });
  });

  it("persists defensive item toggles to the danger_detection section", async () => {
    renderSurvivability();

    fireEvent.click(screen.getByRole("switch", { name: "Black King Bar" }));

    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "danger_detection",
        updates: { auto_bkb: false },
      });
    });
  });

  it("hides the lane phase knobs and zeroes the duration when disabled", async () => {
    renderSurvivability();

    expect(
      screen.getByText("Lane Phase Healing Threshold"),
    ).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("switch", { name: "Use a Lower Threshold Early" }),
    );

    expect(
      screen.queryByText("Lane Phase Healing Threshold"),
    ).not.toBeInTheDocument();

    await vi.advanceTimersByTimeAsync(300);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("update_config", {
        section: "common",
        updates: { lane_phase_duration_seconds: 0 },
      });
    });
  });

  it("keeps detection heuristics on the Danger page only", () => {
    renderSurvivability();

    expect(screen.queryByText("Rapid Loss Threshold")).not.toBeInTheDocument();
    expect(screen.queryByText("Burst Time Window")).not.toBeInTheDocument();
  });
});

describe("Danger Detection page after the survivability split", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    useConfigStore.setState({
      config: JSON.parse(JSON.stringify(mockConfig)),
      loaded: true,
    });
  });

  it("keeps the detection heuristics", () => {
    render(
      <MemoryRouter>
        <DangerDetection />
      </MemoryRouter>,
    );

    expect(screen.getByText("Core Settings")).toBeInTheDocument();
    expect(screen.getByText("Rapid Loss Threshold")).toBeInTheDocument();
  });

  it("leaves the responses to the Survivability page", () => {
    render(
      <MemoryRouter>
        <DangerDetection />
      </MemoryRouter>,
    );

    expect(screen.queryByText("Defensive Items")).not.toBeInTheDocument();
    expect(screen.queryByText("Black King Bar")).not.toBeInTheDocument();
    expect(screen.queryByText("Neutral Items")).not.toBeInTheDocument();
    expect(
      screen.queryByText("Auto-Manta on Silence"),
    ).not.toBeInTheDocument();
  });
});
