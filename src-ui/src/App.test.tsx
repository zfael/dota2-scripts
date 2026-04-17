import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import App from "./App";
import { useActivityStore } from "./stores/activityStore";
import { mockConfig } from "./stores/mockData";
import { useConfigStore } from "./stores/configStore";
import { useGameStore } from "./stores/gameStore";
import { useUIStore } from "./stores/uiStore";
import { useUpdateStore } from "./stores/updateStore";

describe("App status header", () => {
  beforeEach(() => {
    window.history.pushState({}, "", "/");

    useConfigStore.setState({
      config: JSON.parse(JSON.stringify(mockConfig)),
      loaded: true,
    });
    useUIStore.setState({
      invokerActiveComboProfileId: "qw-pickoff",
      appVersion: "0.14.0-rc.9",
    });
    useGameStore.setState((state) => ({
      game: {
        ...state.game,
        heroName: "Invoker",
        heroLevel: 30,
        hpPercent: 100,
        manaPercent: 100,
        connected: true,
      },
    }));
    useActivityStore.setState({ entries: [] });
    useUpdateStore.setState({ updateState: { kind: "idle" } });
  });

  it("shows the active Invoker combo profile in the header", () => {
    render(<App />);

    expect(screen.getByText("Profile: QW Pickoff")).toBeInTheDocument();
  });

  it("shows Profile: None when Invoker has no valid active combo", () => {
    useUIStore.setState({ invokerActiveComboProfileId: "meteor-blast-prep" });

    render(<App />);

    expect(screen.getByText("Profile: None")).toBeInTheDocument();
  });

  it("does not show a profile chip for non-Invoker heroes", () => {
    useGameStore.setState((state) => ({
      game: {
        ...state.game,
        heroName: "Shadow Fiend",
      },
    }));

    render(<App />);

    expect(screen.queryByText(/Profile:/i)).not.toBeInTheDocument();
  });

  it("does not show a profile chip when no game is active", () => {
    useGameStore.setState((state) => ({
      game: {
        ...state.game,
        heroName: null,
      },
    }));

    render(<App />);

    expect(screen.queryByText(/Profile:/i)).not.toBeInTheDocument();
  });
});
