import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useDraftStore } from "./draftStore";
import { EMPTY_DRAFT_STATUS } from "../types/draft";
import type { DraftStatus } from "../types/draft";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("../lib/tauri", () => ({ isTauri: () => true }));

function status(overrides: Partial<DraftStatus> = {}): DraftStatus {
  return {
    ...EMPTY_DRAFT_STATUS,
    enabled: true,
    active: true,
    gameState: "DOTA_GAMERULES_STATE_STRATEGY_TIME",
    sessionId: "1000_0",
    // Bot matches report this as "0" for every single game — which is exactly
    // why it cannot be the reset key.
    matchid: "0",
    slots: [
      {
        index: 0,
        isAlly: true,
        hero: "sven",
        unknown: false,
        agreement: 1,
        bestScore: 0.9,
      },
    ],
    ...overrides,
  };
}

describe("draftStore verdict lifetime", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    useDraftStore.setState({ status: EMPTY_DRAFT_STATUS, judged: {} });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("clears verdicts when a new draft starts, even though matchid never changes", async () => {
    // Regression: `judged` was keyed on matchid. Across two bot matches both
    // report "0", so slot verdicts from game 1 stayed on screen through game 2
    // — the row read "confirmed" and the vote buttons never came back.
    invokeMock.mockResolvedValue(status({ sessionId: "1000_0" }));
    await useDraftStore.getState().fetchStatus();

    invokeMock.mockResolvedValue(undefined);
    await useDraftStore.getState().submitFeedback(0, true);
    expect(useDraftStore.getState().judged[0]).toBe("correct");

    invokeMock.mockResolvedValue(status({ sessionId: "2000_0" }));
    await useDraftStore.getState().fetchStatus();

    expect(useDraftStore.getState().judged).toEqual({});
    expect(useDraftStore.getState().status.sessionId).toBe("2000_0");
  });

  it("keeps verdicts across polls within one draft", async () => {
    invokeMock.mockResolvedValue(status());
    await useDraftStore.getState().fetchStatus();

    invokeMock.mockResolvedValue(undefined);
    await useDraftStore.getState().submitFeedback(0, false, "skeleton_king");
    expect(useDraftStore.getState().judged[0]).toBe("wrong");

    // Same draft, later frame: the verdict must survive.
    invokeMock.mockResolvedValue(status({ frames: 12 }));
    await useDraftStore.getState().fetchStatus();

    expect(useDraftStore.getState().judged[0]).toBe("wrong");
  });

  it("forwards the correction so the reader can harvest that crop", async () => {
    invokeMock.mockResolvedValue(status());
    await useDraftStore.getState().fetchStatus();

    invokeMock.mockReset();
    invokeMock.mockResolvedValue(undefined);
    await useDraftStore.getState().submitFeedback(0, false, "skeleton_king");

    expect(invokeMock).toHaveBeenCalledWith("submit_draft_feedback", {
      slotIndex: 0,
      correct: false,
      actualHero: "skeleton_king",
    });
  });
});
