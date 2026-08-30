import { HPBar } from "../common/HPBar";
import { ManaBar } from "../common/ManaBar";
import { DangerBadge } from "../common/DangerBadge";
import { Badge } from "../common/Badge";
import { Avatar } from "../common/Avatar";
import { HEROES } from "../../types/game";

interface StatusHeaderProps {
  /** Topbar title for the current route. The redesign moved it off the pages. */
  title?: string;
  heroName?: string;
  heroLevel?: number;
  invokerProfileLabel?: string;
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
  title,
  heroName,
  heroLevel,
  invokerProfileLabel,
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
  const hero = HEROES.find(
    (h) => h.displayName.toLowerCase() === heroName?.toLowerCase(),
  );

  return (
    <header className="flex h-14 shrink-0 items-center gap-5 border-b border-border bg-base px-5">
      <span className="text-base font-semibold tracking-[-0.01em] text-content">
        {title ?? "D2 Scripts"}
      </span>

      <div className="flex flex-1 items-center justify-end gap-4">
        <Badge tone={connected ? "success" : "danger"} dot>
          {connected ? "GSI Connected" : "Disconnected"}
        </Badge>

        {inGame ? (
          <>
            <div className="flex items-center gap-3">
              <Avatar
                name={heroName!}
                glyph={hero?.icon}
                size="sm"
                status={connected ? "online" : "offline"}
              />
              <span className="font-semibold text-content">{heroName}</span>
              <span className="font-mono text-2xs text-muted">Lv. {heroLevel}</span>
            </div>

            <div className="w-28">
              <HPBar percent={hpPercent ?? 0} thin />
            </div>
            <div className="w-24">
              <ManaBar percent={manaPercent ?? 0} thin />
            </div>

            {/*
             * State the design's topbar does not cover, but the runtime needs
             * on screen: the automation's own status is worthless if you cannot
             * see why it did nothing.
             */}
            {inDanger && <DangerBadge />}
            {!alive && (
              <Badge tone="danger">
                💀{respawnTimer !== null && ` ${respawnTimer}s`}
              </Badge>
            )}
            {stunned && <Badge tone="warning">⚡ Stunned</Badge>}
            {silenced && <Badge tone="danger">🔇 Silenced</Badge>}
            {runeTimer != null && runeTimer <= 15 && (
              <Badge tone="warning" className="animate-pulse">
                🔮 {runeTimer}s
              </Badge>
            )}
            {invokerProfileLabel && (
              <Badge tone="accent" className="max-w-48" title={invokerProfileLabel}>
                <span className="truncate">{invokerProfileLabel}</span>
              </Badge>
            )}
          </>
        ) : (
          <>
            <span className="text-xs text-muted">Waiting for game...</span>
            <span className="font-mono text-2xs text-muted">v{appVersion}</span>
          </>
        )}
      </div>
    </header>
  );
}
