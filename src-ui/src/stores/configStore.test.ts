import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  CONFIG_UPDATED_EVENT,
  hasPendingConfigWrites,
  useConfigStore,
} from "./configStore";
import { mockConfig } from "./mockData";
import type { Settings } from "../types/config";

const { invokeMock, listenMock, emitEvent, unlistenMock } = vi.hoisted(() => {
  const listeners = new Map<string, (event: { payload: unknown }) => void>();
  const unlistenMock = vi.fn();

  return {
    invokeMock: vi.fn(),
    unlistenMock,
    listenMock: vi.fn(
      async (eventName: string, handler: (event: { payload: unknown }) => void) => {
        listeners.set(eventName, handler);
        return unlistenMock;
      },
    ),
    emitEvent: (eventName: string, payload: unknown) => {
      listeners.get(eventName)?.({ payload });
    },
  };
});

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: listenMock }));
vi.mock("../lib/tauri", () => ({ isTauri: () => true }));

function configWithOpacity(opacity: number): Settings {
  return {
    ...mockConfig,
    wave_overlay: { ...mockConfig.wave_overlay, opacity },
  };
}

describe("configStore live updates", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockClear();
    unlistenMock.mockClear();
    useConfigStore.setState({ config: mockConfig, loaded: false });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("adopts config broadcast by another window", async () => {
    await useConfigStore.getState().startListening();

    emitEvent(CONFIG_UPDATED_EVENT, configWithOpacity(0.42));

    expect(useConfigStore.getState().config.wave_overlay.opacity).toBe(0.42);
    expect(useConfigStore.getState().loaded).toBe(true);
  });

  it("subscribes to the event name Rust emits", async () => {
    await useConfigStore.getState().startListening();

    expect(listenMock).toHaveBeenCalledWith(
      CONFIG_UPDATED_EVENT,
      expect.any(Function),
    );
  });

  it("returns an unsubscribe handle so windows can clean up", async () => {
    const unlisten = await useConfigStore.getState().startListening();
    unlisten();

    expect(unlistenMock).toHaveBeenCalled();
  });

  it("ignores the echo of its own edit while a write is still queued", async () => {
    vi.useFakeTimers();
    try {
      await useConfigStore.getState().startListening();

      // Two sections edited back to back. The first write lands and echoes the
      // settings as they stood before the second one was persisted.
      useConfigStore.getState().updateConfig("wave_overlay", { opacity: 0.5 });
      useConfigStore.getState().updateConfig("alerts", { master_volume: 0.25 });
      expect(hasPendingConfigWrites()).toBe(true);

      emitEvent(CONFIG_UPDATED_EVENT, configWithOpacity(0.5));

      // Without the guard this would have reverted to mockConfig's volume.
      expect(useConfigStore.getState().config.alerts.master_volume).toBe(0.25);
      expect(useConfigStore.getState().config.wave_overlay.opacity).toBe(0.5);
    } finally {
      // Drain the queue: `pendingWrites` is module state and would otherwise leak
      // into the next test.
      await vi.runAllTimersAsync();
      vi.useRealTimers();
    }
  });

  it("accepts broadcasts again once its writes have settled", async () => {
    vi.useFakeTimers();
    try {
      await useConfigStore.getState().startListening();
      invokeMock.mockResolvedValue(undefined);

      useConfigStore.getState().updateConfig("wave_overlay", { opacity: 0.5 });
      await vi.runAllTimersAsync();

      expect(hasPendingConfigWrites()).toBe(false);

      emitEvent(CONFIG_UPDATED_EVENT, configWithOpacity(0.7));
      expect(useConfigStore.getState().config.wave_overlay.opacity).toBe(0.7);
    } finally {
      vi.useRealTimers();
    }
  });
});
