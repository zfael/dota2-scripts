import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useUIStore } from "./uiStore";

const { invokeMock, listenMock, emitEvent } = vi.hoisted(() => {
  const listeners = new Map<string, (event: { payload: unknown }) => void>();

  return {
    invokeMock: vi.fn(),
    listenMock: vi.fn(async (eventName: string, handler: (event: { payload: unknown }) => void) => {
      listeners.set(eventName, handler);
      return () => listeners.delete(eventName);
    }),
    emitEvent: (eventName: string, payload: unknown) => {
      listeners.get(eventName)?.({ payload });
    },
  };
});

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: listenMock,
}));

vi.mock("../lib/tauri", () => ({
  isTauri: () => true,
}));

describe("uiStore armlet Roshan mode", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    listenMock.mockClear();
    useUIStore.setState({
      sidebarCollapsed: false,
      gsiEnabled: true,
      standaloneEnabled: false,
      appVersion: "0.1.0",
      armletRoshanArmed: false,
      invokerActiveComboProfileId: null,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("loads armlet Roshan mode from the app state snapshot", async () => {
    invokeMock.mockResolvedValueOnce({
      selectedHero: null,
      gsiEnabled: true,
      standaloneEnabled: false,
      appVersion: "0.15.0",
      armletRoshanArmed: true,
    });

    await useUIStore.getState().loadInitialState();

    expect(useUIStore.getState().armletRoshanArmed).toBe(true);
    expect(useUIStore.getState().appVersion).toBe("0.15.0");
  });

  it("persists live Roshan mode toggles through the backend command", async () => {
    invokeMock.mockResolvedValue(undefined);

    useUIStore.getState().setArmletRoshanArmed(true);

    expect(useUIStore.getState().armletRoshanArmed).toBe(true);
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_armlet_roshan_mode_armed", { armed: true });
    });
  });

  it("updates live Roshan mode when app_state_update events arrive", async () => {
    const unlisten = await useUIStore.getState().startListening();

    emitEvent("app_state_update", {
      selectedHero: null,
      gsiEnabled: true,
      standaloneEnabled: false,
      appVersion: "0.15.0",
      armletRoshanArmed: true,
    });

    expect(useUIStore.getState().armletRoshanArmed).toBe(true);

    unlisten();
  });

  it("loads and updates the Invoker active combo profile from app state payloads", async () => {
    invokeMock.mockResolvedValueOnce({
      selectedHero: null,
      gsiEnabled: true,
      standaloneEnabled: false,
      appVersion: "0.15.0",
      armletRoshanArmed: false,
      invokerActiveComboProfileId: "qe-burst",
    });

    await useUIStore.getState().loadInitialState();

    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("qe-burst");

    const unlisten = await useUIStore.getState().startListening();

    emitEvent("app_state_update", {
      selectedHero: null,
      gsiEnabled: true,
      standaloneEnabled: false,
      appVersion: "0.15.0",
      armletRoshanArmed: false,
      invokerActiveComboProfileId: "cataclysm-finish",
    });

    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("cataclysm-finish");

    unlisten();
  });

  it("persists active combo profile changes through the backend command", async () => {
    invokeMock.mockResolvedValue(undefined);

    const setInvokerActiveComboProfileId = useUIStore.getState().setInvokerActiveComboProfileId;

    expect(setInvokerActiveComboProfileId).toBeTypeOf("function");
    setInvokerActiveComboProfileId?.("qe-burst");

    expect(useUIStore.getState().invokerActiveComboProfileId).toBe("qe-burst");
    await vi.waitFor(() => {
      expect(invokeMock).toHaveBeenCalledWith("set_invoker_active_combo_profile", {
        profileId: "qe-burst",
      });
    });
  });
});
