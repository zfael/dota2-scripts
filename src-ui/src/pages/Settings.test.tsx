import { beforeEach, describe, expect, it, vi } from "vitest";
import { MemoryRouter } from "react-router-dom";
import { render, screen } from "@testing-library/react";
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

describe("Settings page", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    useConfigStore.setState({ config: mockConfig, loaded: true });
  });

  it("renders the application-wide setting cards", () => {
    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>,
    );

    expect(screen.getByText("Server")).toBeInTheDocument();
    expect(screen.getByText("Keybindings")).toBeInTheDocument();
    expect(screen.getByText("Common")).toBeInTheDocument();
    expect(screen.getByText("Rune Alerts")).toBeInTheDocument();
  });

  it("leaves phase boots configuration to the Boots page", () => {
    render(
      <MemoryRouter>
        <Settings />
      </MemoryRouter>,
    );

    expect(screen.queryByText("Phase Boots")).not.toBeInTheDocument();
    expect(
      screen.queryByText("Enable Phase Boots Automation"),
    ).not.toBeInTheDocument();
  });
});
