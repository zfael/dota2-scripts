import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useStratzStore } from "./stratzStore";
import { EMPTY_DRAFT_ADVICE, EMPTY_STRATZ_STATUS } from "../types/stratz";
import type { StratzStatus } from "../types/stratz";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("../lib/tauri", () => ({ isTauri: () => true }));

function status(overrides: Partial<StratzStatus> = {}): StratzStatus {
  return { ...EMPTY_STRATZ_STATUS, enabled: true, hasToken: true, ready: true, ...overrides };
}

describe("stratzStore token handling", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    useStratzStore.setState({
      status: EMPTY_STRATZ_STATUS,
      advice: EMPTY_DRAFT_ADVICE,
      savingToken: false,
      tokenError: null,
    });
  });

  afterEach(() => vi.clearAllMocks());

  it("reports a rejected token instead of silently accepting it", async () => {
    // The backend validates against the API before saving, so a typo must
    // surface here rather than a minute later as a failed refresh.
    invokeMock.mockRejectedValueOnce("STRATZ rejected the API token");

    const ok = await useStratzStore.getState().saveToken("not-a-real-token");

    expect(ok).toBe(false);
    expect(useStratzStore.getState().tokenError).toContain("rejected");
    expect(useStratzStore.getState().savingToken).toBe(false);
  });

  it("clears the saving flag and error after a successful save", async () => {
    useStratzStore.setState({ tokenError: "previous failure" });
    invokeMock.mockResolvedValueOnce(undefined); // set_stratz_token
    invokeMock.mockResolvedValueOnce(status()); // fetchStatus

    const ok = await useStratzStore.getState().saveToken("eyJvalid");

    expect(ok).toBe(true);
    expect(useStratzStore.getState().tokenError).toBeNull();
    expect(useStratzStore.getState().savingToken).toBe(false);
    expect(useStratzStore.getState().status.hasToken).toBe(true);
    expect(invokeMock).toHaveBeenCalledWith("set_stratz_token", { token: "eyJvalid" });
  });

  it("never keeps a token in store state", async () => {
    invokeMock.mockResolvedValueOnce(undefined);
    invokeMock.mockResolvedValueOnce(status());

    await useStratzStore.getState().saveToken("eyJsupersecret");

    // The credential must not linger anywhere reachable from the webview.
    const serialised = JSON.stringify(useStratzStore.getState());
    expect(serialised).not.toContain("supersecret");
  });

  it("drops stale advice when the token is cleared", async () => {
    useStratzStore.setState({
      advice: {
        suggestions: [
          {
            slug: "axe",
            displayName: "Axe",
            score: 0.1,
            counter: 0.1,
            synergy: 0,
            positionWinRate: null,
            bestAgainst: null,
            counterSamples: 100,
          },
        ],
        unresolved: [],
        alliesUsed: 0,
        enemiesUsed: 1,
      },
    });
    invokeMock.mockResolvedValueOnce(undefined); // clear_stratz_token
    invokeMock.mockResolvedValueOnce(EMPTY_STRATZ_STATUS); // fetchStatus

    await useStratzStore.getState().clearToken();

    expect(useStratzStore.getState().advice.suggestions).toHaveLength(0);
    expect(useStratzStore.getState().status.hasToken).toBe(false);
  });

  it("applies a role change optimistically so the selector feels instant", async () => {
    invokeMock.mockResolvedValue(EMPTY_DRAFT_ADVICE);

    // The optimistic `set` runs synchronously, before the action's first
    // await — so the selector re-renders without waiting on the backend.
    const pending = useStratzStore.getState().setPosition(4);
    expect(useStratzStore.getState().status.position).toBe(4);

    await pending;

    expect(invokeMock).toHaveBeenCalledWith("update_config", {
      section: "stratz",
      updates: { position: 4 },
    });
    // And the advice is recomputed for the new role.
    expect(invokeMock).toHaveBeenCalledWith("get_draft_advice");
  });
});
