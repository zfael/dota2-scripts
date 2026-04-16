import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import InvokerConfig from "./InvokerConfig";
import { useConfigStore } from "../../../stores/configStore";
import { mockConfig } from "../../../stores/mockData";
import { useUIStore } from "../../../stores/uiStore";

vi.mock("../../../lib/tauri", () => ({
  isTauri: () => false,
}));

describe("InvokerConfig", () => {
  beforeEach(() => {
    useConfigStore.setState({
      config: JSON.parse(JSON.stringify(mockConfig)),
      loaded: true,
    });
    useUIStore.setState({
      invokerActiveComboProfileId: null,
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

  it("marks an enabled combo profile as the active combo when clicked", () => {
    render(<InvokerConfig />);

    fireEvent.click(
      screen.getByRole("button", {
        name: /Ghost Walk Panic.*combo.*End.*enabled.*general.*Ghost Walk/i,
      }),
    );

    expect(screen.getByText(/Active combo:/i)).toHaveTextContent(
      "Active combo: Ghost Walk Panic",
    );
    expect(useUIStore.getState().invokerActiveComboProfileId).toBe(
      "ghost-walk-panic",
    );
  });

  it("does not change the active combo when a prep profile is clicked", () => {
    useUIStore.setState({ invokerActiveComboProfileId: "ghost-walk-panic" });

    render(<InvokerConfig />);

    fireEvent.click(
      screen.getByRole("button", {
        name: /Meteor \+ Blast Prep.*prep.*PageUp.*enabled.*qe.*Chaos Meteor → Deafening Blast/i,
      }),
    );

    expect(screen.getByText(/Active combo:/i)).toHaveTextContent(
      "Active combo: Ghost Walk Panic",
    );
    expect(useUIStore.getState().invokerActiveComboProfileId).toBe(
      "ghost-walk-panic",
    );
  });

  it("does not change the active combo when a disabled combo profile is clicked", () => {
    useUIStore.setState({ invokerActiveComboProfileId: "ghost-walk-panic" });

    render(<InvokerConfig />);

    fireEvent.click(
      screen.getByRole("button", {
        name: /QE Burst.*combo.*PageDown.*disabled.*qe.*Sun Strike \[manual\] → Chaos Meteor → Deafening Blast/i,
      }),
    );

    expect(screen.getByText(/Active combo:/i)).toHaveTextContent(
      "Active combo: Ghost Walk Panic",
    );
    expect(useUIStore.getState().invokerActiveComboProfileId).toBe(
      "ghost-walk-panic",
    );
  });

  it("clears the active combo when the active profile is disabled", () => {
    render(<InvokerConfig />);

    fireEvent.click(
      screen.getByRole("button", {
        name: /Ghost Walk Panic.*combo.*End.*enabled.*general.*Ghost Walk/i,
      }),
    );
    fireEvent.click(screen.getByRole("switch", { name: /Enable Profile/i }));

    expect(screen.getByText(/Active combo:/i)).toHaveTextContent(
      "Active combo: None",
    );
    expect(useUIStore.getState().invokerActiveComboProfileId).toBeNull();
  });

  it("clears the active combo when the active profile is deleted", () => {
    render(<InvokerConfig />);

    fireEvent.click(
      screen.getByRole("button", {
        name: /Ghost Walk Panic.*combo.*End.*enabled.*general.*Ghost Walk/i,
      }),
    );
    fireEvent.click(
      screen.getByRole("button", { name: /Delete Ghost Walk Panic/i }),
    );

    expect(screen.getByText(/Active combo:/i)).toHaveTextContent(
      "Active combo: None",
    );
    expect(useUIStore.getState().invokerActiveComboProfileId).toBeNull();
  });
});

