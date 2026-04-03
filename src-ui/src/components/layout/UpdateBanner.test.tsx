import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { UpdateBanner } from "./UpdateBanner";
import { useUpdateStore } from "../../stores/updateStore";

describe("UpdateBanner", () => {
  beforeEach(() => {
    act(() => {
      useUpdateStore.setState({
        updateState: { kind: "checking" },
      });
    });
  });

  afterEach(() => {
    act(() => {
      useUpdateStore.setState({
        updateState: { kind: "idle" },
      });
    });
  });

  it("appears after the startup state transitions from checking to available", () => {
    render(<UpdateBanner />);

    expect(
      screen.queryByText(/Update v0\.15\.0-rc\.2 available/i),
    ).not.toBeInTheDocument();

    act(() => {
      useUpdateStore.setState({
        updateState: {
          kind: "available",
          version: "0.15.0-rc.2",
          releaseNotes: "notes",
        },
      });
    });

    expect(
      screen.getByText(/Update v0\.15\.0-rc\.2 available/i),
    ).toBeInTheDocument();
  });
});
