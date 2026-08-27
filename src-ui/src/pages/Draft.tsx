import { useEffect, useState } from "react";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { useConfigStore } from "../stores/configStore";
import { useDraftStore } from "../stores/draftStore";
import type { DraftSlot } from "../types/draft";

/** `skeleton_king` -> `Skeleton King`, for display only. */
function heroDisplayName(slug: string): string {
  return slug
    .split("_")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
}

function StatusBar() {
  const status = useDraftStore((s) => s.status);

  const stateLabel = status.active
    ? "LIVE — reading draft"
    : status.gameState
      ? status.gameState.replace("DOTA_GAMERULES_STATE_", "")
      : "waiting for Dota";

  return (
    <div className="flex items-center gap-4 rounded-lg border border-border bg-surface px-4 py-2.5 text-sm">
      <div className="flex items-center gap-2">
        <span
          className={`inline-block h-2.5 w-2.5 rounded-full ${
            status.active ? "bg-green-500 animate-pulse" : "bg-muted"
          }`}
        />
        <span className={status.active ? "font-medium text-content" : "text-subtle"}>
          {stateLabel}
        </span>
      </div>
      {status.frames > 0 && (
        <>
          <span className="text-border">|</span>
          <span className="text-subtle">
            Frames: <span className="font-mono text-xs text-content">{status.frames}</span>
          </span>
        </>
      )}
      {status.ownHero && (
        <>
          <span className="text-border">|</span>
          <span className="text-subtle">
            You:{" "}
            <span className="font-medium text-gold">
              {heroDisplayName(status.ownHero.replace("npc_dota_hero_", ""))}
            </span>
          </span>
        </>
      )}
      {status.teamName && (
        <>
          <span className="text-border">|</span>
          <span className="text-subtle capitalize">{status.teamName}</span>
        </>
      )}
    </div>
  );
}

function SlotRow({ slot }: { slot: DraftSlot }) {
  const judged = useDraftStore((s) => s.judged[slot.index]);
  const submitFeedback = useDraftStore((s) => s.submitFeedback);
  const [correcting, setCorrecting] = useState(false);
  const [correction, setCorrection] = useState("");

  const hasContent = slot.hero !== null || slot.unknown;

  return (
    <div className="flex items-center gap-2 rounded-md border border-border bg-elevated px-3 py-2">
      <span className="w-5 shrink-0 font-mono text-xs text-muted">
        {(slot.index % 5) + 1}
      </span>

      <div className="min-w-0 flex-1">
        {slot.hero ? (
          <div>
            <span className="text-sm font-medium text-content">
              {heroDisplayName(slot.hero)}
            </span>
            <span className="ml-2 font-mono text-[10px] text-muted">
              {Math.round(slot.agreement * 100)}%
            </span>
          </div>
        ) : slot.unknown ? (
          // Occupied but unreadable: usually an arcana/persona we have no
          // exemplar for. Honest "?" beats a wrong hero.
          <span className="text-sm font-medium text-amber-400">? unrecognised</span>
        ) : (
          <span className="text-sm text-muted">—</span>
        )}
      </div>

      {!judged && !correcting && (
        <div className="flex shrink-0 gap-1">
          {hasContent && (
            <button
              className="rounded px-2 py-0.5 text-xs text-green-400 hover:bg-green-900/30"
              title="Identification is correct"
              onClick={() => submitFeedback(slot.index, true)}
            >
              ✓
            </button>
          )}
          {/* Also offered on "—" slots: a portrait misread as empty (dark
              portraits once were) can only be reported this way. */}
          <button
            className="rounded px-2 py-0.5 text-xs text-red-400 hover:bg-red-900/30"
            title="This read is wrong — tell us the real hero"
            onClick={() => setCorrecting(true)}
          >
            ✗
          </button>
        </div>
      )}

      {correcting && !judged && (
        <form
          className="flex shrink-0 items-center gap-1"
          onSubmit={(e) => {
            e.preventDefault();
            submitFeedback(slot.index, false, correction.trim() || undefined);
            setCorrecting(false);
          }}
        >
          <input
            autoFocus
            className="w-32 rounded border border-border bg-surface px-2 py-0.5 text-xs text-content placeholder:text-muted"
            placeholder="actual hero…"
            value={correction}
            onChange={(e) => setCorrection(e.target.value)}
          />
          <button className="rounded px-1.5 py-0.5 text-xs text-content hover:bg-surface" type="submit">
            save
          </button>
        </form>
      )}

      {judged && (
        <span
          className={`shrink-0 font-mono text-[10px] ${
            judged === "correct" ? "text-green-500" : "text-red-400"
          }`}
        >
          {judged === "correct" ? "confirmed" : "flagged"}
        </span>
      )}
    </div>
  );
}

function TeamColumn({
  title,
  slots,
  sessionId,
  gold,
}: {
  title: string;
  slots: DraftSlot[];
  sessionId: string;
  gold?: boolean;
}) {
  return (
    <div className="flex-1">
      <h3
        className={`mb-2 text-xs font-semibold uppercase tracking-wider ${
          gold ? "text-gold" : "text-red-400"
        }`}
      >
        {title}
      </h3>
      <div className="space-y-1.5">
        {slots.map((slot) => (
          // Keyed by session so a new draft remounts every row, clearing any
          // half-finished correction left open in the previous one.
          <SlotRow key={`${sessionId}:${slot.index}`} slot={slot} />
        ))}
      </div>
    </div>
  );
}

export default function Draft() {
  const config = useConfigStore((s) => s.config.draft);
  const updateConfig = useConfigStore((s) => s.updateConfig);
  const status = useDraftStore((s) => s.status);
  const startPolling = useDraftStore((s) => s.startPolling);

  useEffect(() => startPolling(), [startPolling]);

  const allies = status.slots.filter((s) => s.isAlly);
  const enemies = status.slots.filter((s) => !s.isAlly);

  return (
    <div className="space-y-4 p-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold text-content">Draft</h1>
          <p className="text-sm text-muted">
            Identifies picked heroes from the draft screen. Capture runs only
            while Dota reports hero selection — never in menus or in game.
          </p>
        </div>
      </div>

      <Card title="Reader">
        <div className="space-y-3">
          <Toggle
            label="Enable draft reading"
            checked={config.enabled}
            onChange={(enabled) => updateConfig("draft", { enabled })}
          />
          {config.enabled && config.telemetry_enabled && (
            <p className="text-xs text-muted">
              Recording captures and reads to{" "}
              <span className="font-mono">{config.telemetry_dir}</span> for
              offline evaluation. Your ✓/✗ votes below are stored with them as
              labels.
            </p>
          )}
        </div>
      </Card>

      {config.enabled && (
        <>
          <StatusBar />

          {status.slots.length > 0 ? (
            <Card title="Lineup">
              <div className="flex gap-6">
                <TeamColumn
                  title="Your team"
                  slots={allies}
                  sessionId={status.sessionId}
                  gold
                />
                <TeamColumn
                  title="Enemy team"
                  slots={enemies}
                  sessionId={status.sessionId}
                />
              </div>
              <p className="mt-3 text-xs text-muted">
                Sanity-check as heroes lock in: ✓ confirms a read, ✗ flags it
                and lets you name the real hero (e.g.{" "}
                <span className="font-mono">skeleton_king</span>). An amber
                &ldquo;?&rdquo; means the portrait could not be matched —
                usually an arcana we have no exemplar for yet.
              </p>
            </Card>
          ) : (
            <div className="rounded-lg border border-border bg-surface px-4 py-8 text-center text-sm text-muted">
              Waiting for a draft. Queue a game — the lineup appears here as
              picks lock in.
            </div>
          )}
        </>
      )}
    </div>
  );
}
