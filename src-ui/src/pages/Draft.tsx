import { useEffect, useMemo, useState } from "react";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { useConfigStore } from "../stores/configStore";
import { useDraftStore } from "../stores/draftStore";
import { useStratzStore } from "../stores/stratzStore";
import type { DraftSlot } from "../types/draft";
import { POSITION_LABELS } from "../types/stratz";

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

/**
 * Step 2 of setup: capture the user's own STRATZ API token.
 *
 * Advice needs a token and each user must bring their own — STRATZ binds a
 * token to an account and rate-limits per token, so one shared key would not
 * work even if shipping a credential were acceptable.
 */
function TokenSetup() {
  const status = useStratzStore((s) => s.status);
  const saveToken = useStratzStore((s) => s.saveToken);
  const savingToken = useStratzStore((s) => s.savingToken);
  const tokenError = useStratzStore((s) => s.tokenError);
  const [token, setToken] = useState("");

  return (
    <div className="space-y-3 rounded-lg border border-gold/40 bg-gold/5 p-4">
      <div>
        <h3 className="text-sm font-semibold text-gold">
          Step 2 — Connect your STRATZ account
        </h3>
        <p className="mt-1 text-xs text-subtle">
          Draft advice reads hero matchup statistics from STRATZ, which needs
          your own free API token.
        </p>
      </div>

      <ol className="ml-4 list-decimal space-y-1 text-xs text-muted">
        <li>
          Open <span className="font-mono text-subtle">stratz.com/api</span> and
          sign in with Steam
        </li>
        <li>
          Choose <span className="text-subtle">My Tokens</span> →{" "}
          <span className="text-subtle">Show Token Information</span>
        </li>
        <li>
          Copy the token (it starts with{" "}
          <span className="font-mono text-subtle">eyJ</span>) and paste it below
        </li>
      </ol>

      <form
        className="flex items-center gap-2"
        onSubmit={async (e) => {
          e.preventDefault();
          if (await saveToken(token.trim())) setToken("");
        }}
      >
        <input
          type="password"
          className="min-w-0 flex-1 rounded border border-border bg-surface px-2 py-1.5 font-mono text-xs text-content placeholder:text-muted"
          placeholder="eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9…"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          spellCheck={false}
          autoComplete="off"
        />
        <button
          type="submit"
          disabled={!token.trim() || savingToken}
          className="shrink-0 rounded bg-gold px-3 py-1.5 text-xs font-medium text-base disabled:opacity-40"
        >
          {savingToken ? "Checking…" : "Save token"}
        </button>
      </form>

      {tokenError && (
        <p className="text-xs text-red-400">{tokenError}</p>
      )}
      <p className="text-[11px] text-muted">
        Stored locally in your config and never sent anywhere but STRATZ. It is
        not shown again after saving.
        {status.hasToken === false &&
          " You can also set STRATZ_TOKEN in your environment instead."}
      </p>
    </div>
  );
}

function RoleSelector() {
  const status = useStratzStore((s) => s.status);
  const setPosition = useStratzStore((s) => s.setPosition);

  return (
    <div className="flex flex-wrap items-center gap-2">
      <span className="text-xs text-subtle">Queuing as:</span>
      {[1, 2, 3, 4, 5].map((p) => (
        <button
          key={p}
          onClick={() => setPosition(status.position === p ? 0 : p)}
          className={`rounded-md border px-2.5 py-1 text-xs transition-colors ${
            status.position === p
              ? "border-gold bg-gold/15 text-gold"
              : "border-border text-subtle hover:border-gold/40 hover:text-content"
          }`}
        >
          {POSITION_LABELS[p]}
        </button>
      ))}
      {status.position === 0 && (
        <span className="text-[11px] text-muted">
          Pick a role — suggestions are ranked for it
        </span>
      )}
    </div>
  );
}

function AdvicePanel() {
  const status = useStratzStore((s) => s.status);
  const advice = useStratzStore((s) => s.advice);
  const draft = useDraftStore((s) => s.status);

  if (status.refreshing) {
    return (
      <Card title="Draft advice">
        <div className="space-y-2">
          <p className="text-sm text-subtle">
            Building the matchup dataset — about a minute, once a day.
          </p>
          <div className="h-1.5 w-full overflow-hidden rounded-full bg-elevated">
            <div
              className="h-full rounded-full bg-gold transition-all"
              style={{ width: `${status.progress}%` }}
            />
          </div>
          <p className="text-xs text-muted">
            Suggestions then come from the local cache, with no network call
            during a draft.
          </p>
        </div>
      </Card>
    );
  }

  if (!status.ready) {
    return (
      <Card title="Draft advice">
        <div className="space-y-2">
          <p className="text-sm text-subtle">
            {status.lastError ?? "Waiting for the matchup dataset."}
          </p>
          {status.lastError && (
            <p className="text-xs text-muted">
              Retrying in the background — nothing to do. Hero identification
              below keeps working regardless.
            </p>
          )}
        </div>
      </Card>
    );
  }

  const built = status.builtAt
    ? new Date(status.builtAt * 1000).toLocaleString()
    : "unknown";

  return (
    <Card title="Draft advice">
      <div className="space-y-3">
        <RoleSelector />

        {advice.suggestions.length === 0 ? (
          <p className="text-sm text-muted">
            {draft.slots.some((s) => s.hero)
              ? "No suggestions for this role."
              : "Suggestions appear as heroes are identified."}
          </p>
        ) : (
          <div className="space-y-1">
            {advice.suggestions.map((s, i) => (
              <div
                key={s.slug}
                className="flex items-center gap-3 rounded-md border border-border bg-elevated px-3 py-2"
              >
                <span className="w-5 shrink-0 font-mono text-xs text-muted">
                  {i + 1}
                </span>
                <span className="min-w-0 flex-1 truncate text-sm font-medium text-content">
                  {s.displayName}
                </span>
                {s.positionWinRate !== null && (
                  <span className="shrink-0 font-mono text-[11px] text-subtle">
                    {(s.positionWinRate * 100).toFixed(1)}% wr
                  </span>
                )}
                {/* Counter and synergy are shown split rather than as one
                    number, so it is clear whether a pick is being suggested
                    against the enemy or alongside your own team. */}
                <span
                  className="shrink-0 font-mono text-[11px] text-green-400"
                  title={`Countering the enemy lineup${
                    s.counterSamples ? ` — ${s.counterSamples.toLocaleString()} games` : ""
                  }`}
                >
                  vs {s.counter >= 0 ? "+" : ""}
                  {(s.counter * 100).toFixed(1)}
                </span>
                {advice.alliesUsed > 0 && (
                  <span
                    className="shrink-0 font-mono text-[11px] text-blue-300"
                    title="Synergy with your existing picks"
                  >
                    with {s.synergy >= 0 ? "+" : ""}
                    {(s.synergy * 100).toFixed(1)}
                  </span>
                )}
                {s.bestAgainst && (
                  <span className="hidden shrink-0 text-[11px] text-muted lg:inline">
                    best vs {s.bestAgainst}
                  </span>
                )}
              </div>
            ))}
          </div>
        )}

        {advice.unresolved.length > 0 && (
          <p className="text-xs text-amber-400">
            Not in the dataset: {advice.unresolved.join(", ")} — the cache
            predates a patch, so these picks are missing from the advice.
          </p>
        )}

        {status.incompleteHeroes > 0 && (
          <p className="text-xs text-amber-400">
            {status.incompleteHeroes} hero
            {status.incompleteHeroes === 1 ? "" : "es"} missing matchup data —
            STRATZ failed those requests while building the cache. They can
            still be suggested, but with no counter or synergy signal. A
            refresh is retried within the hour.
          </p>
        )}

        {/* A cached dataset keeps working while STRATZ is down; say so rather
            than leaving a stale-looking panel with no explanation. */}
        {status.lastError && (
          <p className="text-xs text-amber-400">
            Using the cached dataset — {status.lastError}
          </p>
        )}

        <p className="text-[11px] text-muted">
          {status.heroCount} heroes · {status.bracket.replace("_", " + ")} ·
          built {built} · {advice.enemiesUsed} enemy and {advice.alliesUsed}{" "}
          ally picks counted
        </p>
      </div>
    </Card>
  );
}

export default function Draft() {
  const config = useConfigStore((s) => s.config.draft);
  const updateConfig = useConfigStore((s) => s.updateConfig);
  const status = useDraftStore((s) => s.status);
  const startPolling = useDraftStore((s) => s.startPolling);
  const stratz = useStratzStore((s) => s.status);
  const startStratzPolling = useStratzStore((s) => s.startPolling);
  const fetchAdvice = useStratzStore((s) => s.fetchAdvice);

  useEffect(() => startPolling(), [startPolling]);
  useEffect(() => startStratzPolling(), [startStratzPolling]);

  // Advice is recomputed exactly when the identified lineup changes, rather
  // than on every poll: the ranking depends only on which heroes are known,
  // so re-running it each second would be pure waste.
  const lineupSignature = useMemo(
    () =>
      status.slots
        .map((s) => `${s.index}:${s.hero ?? ""}:${s.isAlly ? "a" : "e"}`)
        .join("|"),
    [status.slots],
  );

  useEffect(() => {
    if (stratz.ready) fetchAdvice();
  }, [lineupSignature, stratz.ready, stratz.position, fetchAdvice]);

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

      <Card title="Setup">
        <div className="space-y-3">
          <div>
            <h3 className="text-sm font-semibold text-content">
              Step 1 — Read the draft
            </h3>
            <p className="mb-2 mt-1 text-xs text-subtle">
              Identifies picked heroes from the draft screen.
            </p>
            <Toggle
              label="Enable draft reading"
              checked={config.enabled}
              onChange={(enabled) => updateConfig("draft", { enabled })}
            />
          </div>

          {config.enabled && config.telemetry_enabled && (
            <p className="text-xs text-muted">
              Recording captures and reads to{" "}
              <span className="font-mono">{config.telemetry_dir}</span> for
              offline evaluation. Your ✓/✗ votes below are stored with them as
              labels.
            </p>
          )}

          {/* Step 2 appears only once the reader is on: a token is no use
              without a draft to advise on. */}
          {config.enabled && !stratz.hasToken && <TokenSetup />}

          {config.enabled && stratz.hasToken && (
            <div className="flex items-center justify-between rounded-lg border border-border bg-elevated px-3 py-2">
              <span className="text-xs text-subtle">
                <span className="text-green-400">✓</span> STRATZ connected
                {stratz.heroCount > 0 && ` · ${stratz.heroCount} heroes cached`}
              </span>
              <button
                className="text-[11px] text-muted hover:text-red-400"
                onClick={() => useStratzStore.getState().clearToken()}
              >
                disconnect
              </button>
            </div>
          )}
        </div>
      </Card>

      {config.enabled && stratz.hasToken && <AdvicePanel />}

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
