import { HPBar } from "../common/HPBar";
import { ManaBar } from "../common/ManaBar";
import { DangerBadge } from "../common/DangerBadge";
import { Wifi, WifiOff } from "lucide-react";

interface StatusHeaderProps {
  heroName?: string;
  heroLevel?: number;
  hpPercent?: number;
  manaPercent?: number;
  inDanger?: boolean;
  connected?: boolean;
  appVersion: string;
  runeTimer?: number | null;
  stunned: boolean;
  silenced: boolean;
  alive: boolean;
  respawnTimer: number | null;
}

export function StatusHeader({
  heroName,
  heroLevel,
  hpPercent,
  manaPercent,
  inDanger = false,
  connected = false,
  appVersion,
  runeTimer,
  stunned,
  silenced,
  alive,
  respawnTimer,
}: StatusHeaderProps) {
  const inGame = !!heroName;

  return (
    <header className="flex h-12 shrink-0 items-center gap-4 border-b border-border bg-surface px-4">
      <div className="flex items-center gap-1.5">
        <span
          className={`inline-block h-2 w-2 rounded-full ${
            connected ? "bg-success" : "bg-danger animate-pulse"
          }`}
        />
        <span className="text-xs text-subtle">
          {connected ? "GSI Connected" : "Disconnected"}
        </span>
      </div>
      {inGame ? (
        <>
          <div className="flex items-center gap-2">
            <span className="font-semibold text-content">{heroName}</span>
            <span className="rounded bg-elevated px-1.5 py-0.5 font-mono text-xs text-subtle">
              Lv. {heroLevel}
            </span>
          </div>
          <div className="flex items-center gap-3 flex-1">
            <div className="w-32">
              <HPBar percent={hpPercent ?? 0} />
            </div>
            <div className="w-28">
              <ManaBar percent={manaPercent ?? 0} />
            </div>
            {inDanger && <DangerBadge />}
            {!alive && (
              <div className="flex items-center gap-1 text-danger text-xs font-mono">
                <span>💀</span>
                {respawnTimer !== null && <span>{respawnTimer}s</span>}
              </div>
            )}
            {stunned && <span className="text-warning text-xs">⚡ Stunned</span>}
            {silenced && <span className="text-danger text-xs">🔇 Silenced</span>}
            {runeTimer != null && runeTimer <= 15 && (
              <span className="font-mono text-xs text-warning animate-pulse">
                🔮 {runeTimer}s
              </span>
            )}
          </div>
          <div className="flex items-center gap-1">
            {connected ? (
              <Wifi className="h-4 w-4 text-success" />
            ) : (
              <WifiOff className="h-4 w-4 text-danger" />
            )}
          </div>
        </>
      ) : (
        <>
          <span className="text-sm font-semibold text-content">D2 Scripts</span>
          <div className="flex items-center gap-2">
            <span className="h-2 w-2 rounded-full bg-subtle animate-pulse" />
            <span className="text-xs text-subtle">Waiting for game...</span>
          </div>
          <div className="flex-1" />
          <span className="text-xs text-muted">v{appVersion}</span>
        </>
      )}
    </header>
  );
}
