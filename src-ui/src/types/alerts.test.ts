import { describe, it, expect } from "vitest";
import { ALERT_EVENTS, formatCountdown } from "./alerts";

describe("formatCountdown", () => {
  it("formats minutes and seconds", () => {
    expect(formatCountdown(0)).toBe("0:00");
    expect(formatCountdown(75)).toBe("1:15");
    expect(formatCountdown(360)).toBe("6:00");
  });

  it("shows a dash when a fixed schedule has run out", () => {
    // Water runes stop after 4:00, so there is no next occurrence.
    expect(formatCountdown(null)).toBe("—");
  });

  it("reads as now rather than a negative time", () => {
    expect(formatCountdown(-2)).toBe("now");
  });
});

describe("ALERT_EVENTS", () => {
  it("covers every event exactly once", () => {
    const keys = ALERT_EVENTS.map((e) => e.key);
    expect(keys).toHaveLength(7);
    expect(new Set(keys).size).toBe(7);
  });

  it("describes the cue for every event so the mapping can be learned", () => {
    for (const event of ALERT_EVENTS) {
      expect(event.cue.length).toBeGreaterThan(0);
      expect(event.schedule.length).toBeGreaterThan(0);
    }
  });
});
