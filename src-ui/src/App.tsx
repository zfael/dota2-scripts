import { useEffect, useRef } from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
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
import DangerDetection from "./pages/DangerDetection";
import SoulRing from "./pages/SoulRing";
import Armlet from "./pages/Armlet";
import ActivityLog from "./pages/ActivityLog";
import Diagnostics from "./pages/Diagnostics";
import Settings from "./pages/Settings";
import MinimapIntelligence from "./pages/MinimapIntelligence";

function useRuneAlert(runeTimer: number | null) {
  const lastAlertRef = useRef<number | null>(null);

  useEffect(() => {
    if (runeTimer === null || runeTimer > 10) {
      lastAlertRef.current = null;
      return;
    }

    // Don't re-alert for the same rune window
    if (lastAlertRef.current !== null && lastAlertRef.current <= 10) {
      return;
    }

    lastAlertRef.current = runeTimer;

    // Play a short alert tone using Web Audio API
    try {
      const ctx = new AudioContext();
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.frequency.value = 880;
      gain.gain.value = 0.15;
      osc.start();
      osc.stop(ctx.currentTime + 0.12);
      setTimeout(() => ctx.close(), 500);
    } catch {
      // AudioContext may not be available
    }
  }, [runeTimer]);
}

export default function App() {
  useEffect(() => {
    useConfigStore.getState().loadConfig();
    useUIStore.getState().loadInitialState();
    const gameUnlistenPromise = useGameStore.getState().startListening();
    const activityUnlistenPromise = useActivityStore.getState().startListening();
    useUpdateStore.getState().loadInitialState();

    return () => {
      gameUnlistenPromise.then((unlisten) => unlisten());
      activityUnlistenPromise.then((unlisten) => unlisten());
    };
  }, []);

  const game = useGameStore((s) => s.game);
  const appVersion = useUIStore((s) => s.appVersion);
  useRuneAlert(game.runeTimer);
  const entries = useActivityStore((s) => s.entries);
  const tickerEntries = entries.slice(-3).map((e) => ({
    id: e.id,
    timestamp: e.timestamp,
    category: e.category as "action" | "danger" | "warning" | "system",
    message: e.message,
  }));

  return (
    <BrowserRouter>
      <div className="flex h-screen w-screen overflow-hidden bg-base">
        <Sidebar />
        <div className="flex flex-1 flex-col overflow-hidden">
          <StatusHeader
            heroName={game.heroName ?? undefined}
            heroLevel={game.heroLevel}
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
              <Route path="/danger" element={<DangerDetection />} />
              <Route path="/soul-ring" element={<SoulRing />} />
              <Route path="/armlet" element={<Armlet />} />
              <Route path="/activity" element={<ActivityLog />} />
              <Route path="/minimap" element={<MinimapIntelligence />} />
              <Route path="/diagnostics" element={<Diagnostics />} />
              <Route path="/settings" element={<Settings />} />
            </Routes>
          </main>
          <ActivityTicker entries={tickerEntries} />
        </div>
      </div>
    </BrowserRouter>
  );
}
