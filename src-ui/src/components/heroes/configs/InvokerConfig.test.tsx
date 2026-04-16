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
});

