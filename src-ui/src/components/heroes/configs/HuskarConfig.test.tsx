import { beforeEach, describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import HuskarConfig from "./HuskarConfig";
import { useConfigStore } from "../../../stores/configStore";
import { mockConfig } from "../../../stores/mockData";

vi.mock("../../../lib/tauri", () => ({
  isTauri: () => false,
}));

describe("Huskar Roshan Spears config", () => {
  beforeEach(() => {
    useConfigStore.setState({
      config: mockConfig,
      loaded: true,
    });
  });

  it("renders Roshan Spears controls and updates hero config", () => {
    render(<HuskarConfig />);

    expect(screen.getByText("Roshan Spears")).toBeInTheDocument();
    expect(
      screen.getByRole("switch", { name: "Enable Roshan Spears Gate" }),
    ).toHaveAttribute("aria-checked", "false");
    expect(screen.getByText("Burning Spears Key")).toBeInTheDocument();
    expect(screen.getByText("Disable Buffer")).toBeInTheDocument();
    expect(screen.getByText("Re-enable Buffer")).toBeInTheDocument();

    fireEvent.click(
      screen.getByRole("switch", { name: "Enable Roshan Spears Gate" }),
    );

    expect(
      useConfigStore.getState().config.heroes.huskar.roshan_spears.enabled,
    ).toBe(true);
  });
});
