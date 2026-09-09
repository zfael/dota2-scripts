import { useEffect, useMemo } from "react";
import { MemoryRouter, Routes, Route } from "react-router-dom";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusHeader } from "./components/layout/StatusHeader";
import { UpdateBanner } from "./components/layout/UpdateBanner";
import { ActivityTicker } from "./components/layout/ActivityTicker";
import { useConfigStore } from "./stores/configStore";
import { useGameStore } from "./stores/gameStore";
import { useUIStore } from "./stores/uiStore";
import { useUpdateStore } from "./stores/updateStore";
import { useActivityStore } from "./stores/activityStore";
import Dashboard from "./pages/Dashboard";
import Heroes from "./pages/Heroes";
import HeroDetail from "./pages/HeroDetail";
import Survivability from "./pages/Survivability";
import DangerDetection from "./pages/DangerDetection";
import SoulRing from "./pages/SoulRing";
import Armlet from "./pages/Armlet";
import Boots from "./pages/Boots";
import ActivityLog from "./pages/ActivityLog";
import Diagnostics from "./pages/Diagnostics";
import Settings from "./pages/Settings";
import MinimapIntelligence from "./pages/MinimapIntelligence";
import WaveTracker from "./pages/WaveTracker";
import Alerts from "./pages/Alerts";

/**
 * Stop the webview treating the thumb mouse buttons as Back and Forward.
 *
 * `MemoryRouter` already leaves the gesture with no in-app history to pop, but
 * the webview can still act on the buttons itself. Swallowing them here is also
 * what lets `KeyInput` capture Mouse4/Mouse5 as ordinary bindings.
 *
 * Registered as a capturing listener on `window` so it runs before anything
 * else, and paired with `auxclick` because preventing `mousedown` alone does
 * not stop every navigation path.
 */
function useSuppressBackForwardButtons() {
  useEffect(() => {
    const swallowThumbButtons = (event: MouseEvent) => {
      // 3 = back (Mouse4), 4 = forward (Mouse5). Left/middle/right pass through.
      if (event.button === 3 || event.button === 4) {
        event.preventDefault();
      }
    };

    const events: Array<keyof WindowEventMap> = ["mousedown", "mouseup", "auxclick"];
    events.forEach((name) =>
      window.addEventListener(name, swallowThumbButtons as EventListener, { capture: true }),
    );

    return () =>
      events.forEach((name) =>
        window.removeEventListener(name, swallowThumbButtons as EventListener, {
          capture: true,
        }),
      );
  }, []);
}

export default function App() {
  useSuppressBackForwardButtons();

  useEffect(() => {
    useConfigStore.getState().loadConfig();
    useUIStore.getState().loadInitialState();
    const uiUnlistenPromise = useUIStore.getState().startListening();
    const gameUnlistenPromise = useGameStore.getState().startListening();
    const activityUnlistenPromise = useActivityStore.getState().startListening();
    const configUnlistenPromise = useConfigStore.getState().startListening();
    useUpdateStore.getState().loadInitialState();

    return () => {
      uiUnlistenPromise.then((unlisten) => unlisten());
      gameUnlistenPromise.then((unlisten) => unlisten());
      activityUnlistenPromise.then((unlisten) => unlisten());
      configUnlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const game = useGameStore((s) => s.game);
  const appVersion = useUIStore((s) => s.appVersion);
  const invokerActiveComboProfileId = useUIStore(
    (s) => s.invokerActiveComboProfileId,
  );
  const invokerProfiles = useConfigStore((s) => s.config.heroes.invoker.profiles);
  // Alert audio is played by the Rust engine (see src/observability/alerts.rs),
  // not here: this window is normally minimised while playing, and the OS may
  // throttle a backgrounded WebView's timers and suspend its AudioContext.
  const entries = useActivityStore((s) => s.entries);
  const tickerEntries = entries.slice(-3).map((e) => ({
    id: e.id,
    timestamp: e.timestamp,
    category: e.category as "action" | "danger" | "warning" | "system",
    message: e.message,
  }));
  const invokerProfileLabel = useMemo(() => {
    if (game.heroName !== "Invoker") {
      return undefined;
    }

    const activeProfile = invokerProfiles.find(
      (profile) =>
        profile.id === invokerActiveComboProfileId &&
        profile.mode === "combo" &&
        profile.enabled,
    );

    return `Profile: ${activeProfile?.name ?? "None"}`;
  }, [game.heroName, invokerActiveComboProfileId, invokerProfiles]);

  return (
    /* MemoryRouter, not BrowserRouter: BrowserRouter pushes every navigation
       onto the real WebView2 history, and Mouse4 is the webview's built-in Back
       gesture. Binding an item slot to Mouse4 therefore threw the user back to
       the Dashboard instead of registering the key. Keeping history in memory
       leaves the gesture nothing to pop. Nothing here needs real URLs. */
    <MemoryRouter>
      <div className="flex h-screen w-screen overflow-hidden bg-base">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <StatusHeader
            heroName={game.heroName ?? undefined}
            heroLevel={game.heroLevel}
            invokerProfileLabel={invokerProfileLabel}
            hpPercent={game.hpPercent}
            manaPercent={game.manaPercent}
            inDanger={game.inDanger}
            connected={game.connected}
            appVersion={appVersion}
            runeTimer={game.runeTimer}
            stunned={game.stunned}
            silenced={game.silenced}
            alive={game.alive}
            respawnTimer={game.respawnTimer}
          />
          <UpdateBanner />
          <main className="flex-1 overflow-y-auto page-transition">
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/heroes" element={<Heroes />} />
              <Route path="/heroes/:heroId" element={<HeroDetail />} />
              <Route path="/survivability" element={<Survivability />} />
              <Route path="/danger" element={<DangerDetection />} />
              <Route path="/soul-ring" element={<SoulRing />} />
              <Route path="/armlet" element={<Armlet />} />
              <Route path="/boots" element={<Boots />} />
              <Route path="/activity" element={<ActivityLog />} />
              <Route path="/minimap" element={<MinimapIntelligence />} />
              <Route path="/waves" element={<WaveTracker />} />
              <Route path="/alerts" element={<Alerts />} />
              <Route path="/diagnostics" element={<Diagnostics />} />
              <Route path="/settings" element={<Settings />} />
            </Routes>
          </main>
          <ActivityTicker entries={tickerEntries} />
        </div>
      </div>
    </MemoryRouter>
  );
}
