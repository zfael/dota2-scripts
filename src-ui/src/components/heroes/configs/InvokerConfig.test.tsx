import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import InvokerConfig from "./InvokerConfig";
import { useConfigStore } from "../../../stores/configStore";
import { mockConfig } from "../../../stores/mockData";

vi.mock("../../../lib/tauri", () => ({
  isTauri: () => false,
}));

describe("InvokerConfig", () => {
  beforeEach(() => {
    useConfigStore.setState({
      config: JSON.parse(JSON.stringify(mockConfig)),
      loaded: true,
    });
  });

  it("renders invoker profiles as editable cards", () => {
    render(<InvokerConfig />);

    expect(screen.getAllByText("QW Pickoff").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Tornado").length).toBeGreaterThan(0);
    expect(screen.getAllByText("EMP").length).toBeGreaterThan(0);
  });

  it("lets the user duplicate a preset profile", () => {
    render(<InvokerConfig />);

    fireEvent.click(
      screen.getByRole("button", { name: /duplicate qw pickoff/i }),
    );

    expect(screen.getAllByText(/QW Pickoff/i).length).toBeGreaterThan(1);
  });

  it("renders QE Burst with manual cooldown wait controls", () => {
    render(<InvokerConfig />);

    fireEvent.click(screen.getByText(/PageDown/).closest("button")!);

    expect(screen.getAllByText("Completion Mode").length).toBeGreaterThan(0);
    expect(screen.getByDisplayValue("3000")).toBeInTheDocument();
  });

  it("persists completion mode edits into the config store", () => {
    render(<InvokerConfig />);

    fireEvent.click(screen.getByText(/PageDown/).closest("button")!);
    fireEvent.change(screen.getAllByDisplayValue("Wait for Cooldown")[0], {
      target: { value: "fixed_delay" },
    });

    const qeProfile = useConfigStore
      .getState()
      .config.heroes.invoker.profiles.find((profile) => profile.id === "qe-burst");

    expect(qeProfile?.steps[0].completion_mode).toBe("fixed_delay");
  });
});

