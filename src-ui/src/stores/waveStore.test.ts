import { describe, it, expect } from "vitest";
import { interpolatedClock, MAX_CLOCK_DRIFT_SECONDS } from "./waveStore";
import { formatGameClock } from "../types/waves";

describe("interpolatedClock", () => {
  it("advances smoothly between GSI packets", () => {
    expect(interpolatedClock(100, 0, 500)).toBeCloseTo(100.5, 3);
    expect(interpolatedClock(100, 0, 900)).toBeCloseTo(100.9, 3);
  });

  it("returns the anchor exactly when no time has passed", () => {
    expect(interpolatedClock(42, 1000, 1000)).toBe(42);
  });

  it("freezes once drift exceeds the cap, so a paused game stops the clock", () => {
    // The game is paused: gameTime stays put while wall time keeps running.
    const atCap = interpolatedClock(100, 0, MAX_CLOCK_DRIFT_SECONDS * 1000);
    const wellPast = interpolatedClock(100, 0, 60_000);

    expect(atCap).toBeCloseTo(100 + MAX_CLOCK_DRIFT_SECONDS, 3);
    expect(wellPast).toBe(atCap);
  });

  it("never runs backwards if the clock source jitters", () => {
    expect(interpolatedClock(100, 5000, 4000)).toBe(100);
  });

  it("handles pre-horn negative clocks", () => {
    expect(interpolatedClock(-30, 0, 1000)).toBeCloseTo(-29, 3);
  });
});

describe("formatGameClock", () => {
  it("formats whole minutes and seconds", () => {
    expect(formatGameClock(0)).toBe("0:00");
    expect(formatGameClock(65)).toBe("1:05");
    expect(formatGameClock(600)).toBe("10:00");
  });

  it("truncates fractional seconds rather than rounding up", () => {
    expect(formatGameClock(59.9)).toBe("0:59");
  });

  it("marks pre-horn time as negative", () => {
    expect(formatGameClock(-75)).toBe("-1:15");
  });
});
