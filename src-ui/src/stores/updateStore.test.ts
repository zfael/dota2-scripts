import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUpdateStore } from "./updateStore";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("../lib/tauri", () => ({
  isTauri: () => true,
}));

describe("updateStore startup refresh", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    vi.useFakeTimers();
    useUpdateStore.setState({
      updateState: { kind: "idle" },
    });
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
  });

  it("keeps polling startup state until an update becomes available", async () => {
    invokeMock
      .mockResolvedValueOnce({ kind: "checking" })
      .mockResolvedValueOnce({
        kind: "available",
        version: "0.15.0-rc.2",
        releaseNotes: "notes",
      });

    const load = useUpdateStore.getState().loadInitialState();

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(750);
    await load;

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(useUpdateStore.getState().updateState).toEqual({
      kind: "available",
      version: "0.15.0-rc.2",
      releaseNotes: "notes",
    });
  });

  it("stops immediately when the first startup snapshot is already terminal", async () => {
    invokeMock.mockResolvedValueOnce({ kind: "upToDate" });

    await useUpdateStore.getState().loadInitialState();

    expect(invokeMock).toHaveBeenCalledTimes(1);
    expect(useUpdateStore.getState().updateState).toEqual({ kind: "upToDate" });
  });

  it("stops after the bounded retry budget when startup stays checking", async () => {
    invokeMock.mockResolvedValue({ kind: "checking" });

    const load = useUpdateStore.getState().loadInitialState();

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(750 * 20);
    await load;

    expect(invokeMock).toHaveBeenCalledTimes(21);
    expect(useUpdateStore.getState().updateState).toEqual({ kind: "checking" });
  });

  it("stops polling when the backend returns an error state", async () => {
    invokeMock
      .mockResolvedValueOnce({ kind: "checking" })
      .mockResolvedValueOnce({ kind: "error", message: "network down" });

    const load = useUpdateStore.getState().loadInitialState();

    await Promise.resolve();
    await vi.advanceTimersByTimeAsync(750);
    await load;

    expect(invokeMock).toHaveBeenCalledTimes(2);
    expect(useUpdateStore.getState().updateState).toEqual({
      kind: "error",
      message: "network down",
    });
  });
});
