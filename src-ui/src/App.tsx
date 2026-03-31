import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Sidebar } from "./components/layout/Sidebar";
import { StatusHeader } from "./components/layout/StatusHeader";
import { ActivityTicker } from "./components/layout/ActivityTicker";
import { useGameStore } from "./stores/gameStore";
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

export default function App() {
  const game = useGameStore((s) => s.game);
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
            runeTimer={game.runeTimer}
            stunned={game.stunned}
            silenced={game.silenced}
            alive={game.alive}
            respawnTimer={game.respawnTimer}
          />
          <main className="flex-1 overflow-y-auto">
            <Routes>
              <Route path="/" element={<Dashboard />} />
              <Route path="/heroes" element={<Heroes />} />
              <Route path="/heroes/:heroId" element={<HeroDetail />} />
              <Route path="/danger" element={<DangerDetection />} />
              <Route path="/soul-ring" element={<SoulRing />} />
              <Route path="/armlet" element={<Armlet />} />
              <Route path="/activity" element={<ActivityLog />} />
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
