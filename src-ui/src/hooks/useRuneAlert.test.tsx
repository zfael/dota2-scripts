import { render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useRuneAlert } from "./useRuneAlert";

class FakeOscillator {
  frequency = { value: 0 };
  connect = vi.fn();
  start = vi.fn();
  stop = vi.fn();
}

class FakeGainNode {
  gain = { value: 0 };
  connect = vi.fn();
}

class FakeAudioContext {
  static instances = 0;
  currentTime = 0;
  destination = {};

  constructor() {
    FakeAudioContext.instances += 1;
  }

  createOscillator() {
    return new FakeOscillator();
  }

  createGain() {
    return new FakeGainNode();
  }

  close = vi.fn().mockResolvedValue(undefined);
}

function Harness(props: {
  runeTimer: number | null;
  alertsEnabled: boolean;
  audioEnabled: boolean;
}) {
  useRuneAlert(props.runeTimer, props.alertsEnabled, props.audioEnabled);
  return null;
}

describe("useRuneAlert", () => {
  beforeEach(() => {
    FakeAudioContext.instances = 0;
    vi.stubGlobal("AudioContext", FakeAudioContext);
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it("does not play audio when rune alerts are disabled", () => {
    render(<Harness runeTimer={10} alertsEnabled={false} audioEnabled={true} />);
    expect(FakeAudioContext.instances).toBe(0);
  });

  it("does not play audio when audio is disabled", () => {
    render(<Harness runeTimer={10} alertsEnabled={true} audioEnabled={false} />);
    expect(FakeAudioContext.instances).toBe(0);
  });

  it("plays only once within the same rune window", () => {
    const { rerender } = render(
      <Harness runeTimer={10} alertsEnabled={true} audioEnabled={true} />,
    );

    rerender(<Harness runeTimer={9} alertsEnabled={true} audioEnabled={true} />);

    expect(FakeAudioContext.instances).toBe(1);
  });
});
