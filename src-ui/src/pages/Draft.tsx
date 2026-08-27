import { useEffect, useMemo, useState } from "react";
import { Card } from "../components/common/Card";
import { Toggle } from "../components/common/Toggle";
import { heroName, heroPortraitUrl } from "../lib/heroes";
import { useConfigStore } from "../stores/configStore";
import { useDraftStore } from "../stores/draftStore";
import { useStratzStore } from "../stores/stratzStore";
import type { DraftSlot } from "../types/draft";
import type { MatchupDetail, Suggestion } from "../types/stratz";
import { POSITION_LABELS, POSITION_SHORT } from "../types/stratz";

/**
 * Valve's portrait for a hero, falling back to the name if it cannot load.
 *
 * The art is fetched from Steam's CDN rather than bundled, so an offline
 * client — or a hero too new for the CDN — has to degrade to something
 * readable instead of a broken image.
 */
function HeroPortrait({
  slug,
  className = "h-8 w-[57px]",
}: {
  slug: string;
  className?: string;
}) {
  const [failed, setFailed] = useState(false);

  if (failed) {
    return (
      <div
        className={`${className} flex shrink-0 items-center justify-center rounded border border-border bg-elevated text-[9px] font-medium uppercase text-muted`}
        title={heroName(slug)}
      >
        {heroName(slug).slice(0, 3)}
      </div>
    );
  }

  return (
    <img
      src={heroPortraitUrl(slug)}
      alt={heroName(slug)}
      title={heroName(slug)}
      loading="lazy"
      onError={() => setFailed(true)}
      className={`${className} shrink-0 rounded border border-border object-cover`}
    />
  );
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
            You: <span className="font-medium text-gold">{heroName(status.ownHero)}</span>
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

      {slot.hero && <HeroPortrait slug={slot.hero} className="h-7 w-[50px]" />}

      <div className="min-w-0 flex-1">
        {slot.hero ? (
          <div>
            <span className="text-sm font-medium text-content">{heroName(slot.hero)}</span>
            <span
              className="ml-2 font-mono text-[10px] text-muted"
              title="How consistently the frames agreed on this hero"
            >
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

/** A signed win-rate offset, coloured by which way it points. */
function MatchupCell({ detail, kind }: { detail: MatchupDetail; kind: "vs" | "with" }) {
  if (detail.matches === 0) {
    return (
      <span
        className="text-center font-mono text-[11px] text-muted"
        title={`No recorded games ${kind === "vs" ? "against" : "alongside"} ${detail.displayName}`}
      >
        —
      </span>
    );
  }

  const points = detail.offset * 100;
  const good = points >= 0;
  return (
    <span
      className={`text-center font-mono text-[11px] ${good ? "text-green-400" : "text-red-400"}`}
      title={
        `${good ? "+" : ""}${points.toFixed(1)} points ${
          kind === "vs" ? "better than its own average into" : "better than its own average with"
        } ${detail.displayName}, over ${detail.matches.toLocaleString()} games. ` +
        `Counts as ${detail.contribution >= 0 ? "+" : ""}${(detail.contribution * 100).toFixed(
          1,
        )} after weighting for that sample.`
      }
    >
      {good ? "+" : ""}
      {points.toFixed(1)}
    </span>
  );
}

/**
 * One ranked pick as a row of the matchup table.
 *
 * The previous panel gave three summed numbers per hero and one "best vs"
 * line — which named the same enemy for nine of twelve picks, so it carried
 * nothing. Here every enemy gets its own column: the ranking is visible as
 * evidence rather than asserted.
 */
function SuggestionRow({
  rank,
  suggestion,
  topScore,
  columns,
  position,
}: {
  rank: number;
  suggestion: Suggestion;
  topScore: number;
  columns: string;
  position: number;
}) {
  const s = suggestion;
  // The bar is relative to the best pick on offer; the raw score is a sum of
  // win-rate offsets and means nothing on its own.
  const fit = topScore > 0 ? Math.max(0.04, Math.min(1, s.score / topScore)) : 0;
  const role = POSITION_SHORT[position];

  return (
    <div
      className="grid items-center gap-x-2 rounded-md border border-border bg-elevated px-2 py-1.5"
      style={{ gridTemplateColumns: columns }}
    >
      <span className="text-center font-mono text-xs text-muted">{rank}</span>
      <HeroPortrait slug={s.slug} className="h-7 w-[50px]" />
      <span className="truncate text-sm font-medium text-content" title={s.displayName}>
        {s.displayName}
      </span>

      <span
        className="text-right font-mono text-[11px] text-subtle"
        title={
          s.positionWinRate === null
            ? "No measured win rate for this role"
            : `Win rate${role ? ` as ${role}` : ""} in this bracket`
        }
      >
        {s.positionWinRate === null ? "—" : `${(s.positionWinRate * 100).toFixed(1)}%`}
      </span>

      <span
        className="text-right font-mono text-[11px] text-subtle"
        title={
          s.pickRate === null
            ? "Popularity unknown — STRATZ never returned this hero's matchups"
            : `Picked in ${(s.pickRate * 100).toFixed(1)}% of matches${
                role ? ` as ${role}` : ""
              }`
        }
      >
        {s.pickRate === null ? "—" : `${(s.pickRate * 100).toFixed(1)}%`}
      </span>

      <span
        className="flex items-center"
        title={
          `Fit relative to the best pick on offer. Counters ${
            s.counter >= 0 ? "+" : ""
          }${(s.counter * 100).toFixed(1)}, synergy ${s.synergy >= 0 ? "+" : ""}${(
            s.synergy * 100
          ).toFixed(1)}, over ${s.counterSamples.toLocaleString()} matchup games.`
        }
      >
        <span className="h-1.5 w-full overflow-hidden rounded-full bg-surface">
          <span
            className="block h-full rounded-full bg-gold"
            style={{ width: `${fit * 100}%` }}
          />
        </span>
      </span>

      {s.vsEnemies.map((d) => (
        <MatchupCell key={`vs-${d.slug}`} detail={d} kind="vs" />
      ))}
      {s.withAllies.map((d) => (
        <MatchupCell key={`with-${d.slug}`} detail={d} kind="with" />
      ))}
    </div>
  );
}

/** Column headings: the enemies and allies each column is measured against. */
function SuggestionHeader({
  sample,
  columns,
}: {
  sample: Suggestion;
  columns: string;
}) {
  const vsSpan = sample.vsEnemies.length;
  const withSpan = sample.withAllies.length;

  return (
    <div className="space-y-1">
      {(vsSpan > 0 || withSpan > 0) && (
        <div className="grid gap-x-2 px-2" style={{ gridTemplateColumns: columns }}>
          <span className="col-span-6" />
          {vsSpan > 0 && (
            <span
              className="text-center text-[10px] uppercase tracking-wider text-red-400/80"
              style={{ gridColumn: `span ${vsSpan}` }}
            >
              against
            </span>
          )}
          {withSpan > 0 && (
            <span
              className="text-center text-[10px] uppercase tracking-wider text-blue-300/80"
              style={{ gridColumn: `span ${withSpan}` }}
            >
              with
            </span>
          )}
        </div>
      )}

      <div
        className="grid items-end gap-x-2 px-2 pb-1"
        style={{ gridTemplateColumns: columns }}
      >
        <span />
        <span />
        <span className="text-[10px] uppercase tracking-wider text-muted">hero</span>
        <span className="text-right text-[10px] uppercase tracking-wider text-muted">win</span>
        <span className="text-right text-[10px] uppercase tracking-wider text-muted">picked</span>
        <span className="text-[10px] uppercase tracking-wider text-muted">fit</span>
        {[...sample.vsEnemies, ...sample.withAllies].map((d, i) => (
          <span key={`${d.slug}-${i}`} className="flex justify-center">
            <HeroPortrait slug={d.slug} className="h-6 w-[42px]" />
          </span>
        ))}
      </div>
    </div>
  );
}

function AdviceList({ suggestions, position }: { suggestions: Suggestion[]; position: number }) {
  const sample = suggestions[0];
  const cells = sample.vsEnemies.length + sample.withAllies.length;
  // `repeat(0, ...)` is not valid CSS, and the first suggestions arrive before
  // any hero is identified — an invalid template would drop the whole grid.
  const columns =
    "1.25rem 3.25rem minmax(7rem, 1fr) 3.5rem 3.5rem 4rem" +
    (cells > 0 ? ` repeat(${cells}, 3.25rem)` : "");
  const topScore = sample.score;
  // A wide draft (five enemies and four allies) exceeds a narrow window;
  // scroll the table rather than crushing the hero names.
  const minWidth = 420 + cells * 60;

  return (
    <div className="overflow-x-auto">
      <div style={{ minWidth }}>
        <SuggestionHeader sample={sample} columns={columns} />
        <div className="space-y-1">
          {suggestions.map((s, i) => (
            <SuggestionRow
              key={s.slug}
              rank={i + 1}
              suggestion={s}
              topScore={topScore}
              columns={columns}
              position={position}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

/** "3 hours ago" — the form that answers "is this stale?" at a glance. */
function timeAgo(unixSeconds: number): string {
  const minutes = Math.max(0, Math.round((Date.now() / 1000 - unixSeconds) / 60));
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 48) return `${hours} hour${hours === 1 ? "" : "s"} ago`;
  return `${Math.round(hours / 24)} days ago`;
}

/** Progress of a rebuild, as a strip that does not displace the advice. */
function RefreshProgress({ progress }: { progress: number }) {
  return (
    <div className="space-y-1 rounded-md border border-gold/30 bg-gold/5 px-3 py-2">
      <p className="text-xs text-gold">
        Rebuilding the matchup dataset — about a minute at the free rate limit.
      </p>
      <div className="h-1.5 w-full overflow-hidden rounded-full bg-elevated">
        <div
          className="h-full rounded-full bg-gold transition-all"
          style={{ width: `${progress}%` }}
        />
      </div>
    </div>
  );
}

/** Manual rebuild, for when the 24h cache has not expired but a patch landed. */
function RefreshButton() {
  const status = useStratzStore((s) => s.status);
  const requesting = useStratzStore((s) => s.requestingRefresh);
  const refreshDataset = useStratzStore((s) => s.refreshDataset);
  const busy = requesting || status.refreshing;

  return (
    <button
      onClick={() => refreshDataset()}
      disabled={busy}
      title={
        busy
          ? "A rebuild is already running"
          : "Fetch the current matchup data from STRATZ now — about a minute"
      }
      className="rounded border border-border px-2 py-0.5 text-[11px] text-subtle transition-colors hover:border-gold/50 hover:text-gold disabled:cursor-not-allowed disabled:opacity-40"
    >
      {busy ? "Refreshing…" : "Refresh now"}
    </button>
  );
}

function AdvicePanel() {
  const status = useStratzStore((s) => s.status);
  const advice = useStratzStore((s) => s.advice);
  const setMetaOnly = useStratzStore((s) => s.setMetaOnly);
  const refreshError = useStratzStore((s) => s.refreshError);
  const draft = useDraftStore((s) => s.status);

  // Only when there is nothing to show. A rebuild started mid-draft used to
  // replace the whole panel for a minute, taking the advice off screen at the
  // exact moment it is needed — with a dataset loaded, the old numbers stay
  // up and the progress strip goes above them.
  if (status.refreshing && !status.ready) {
    return (
      <Card title="Draft advice">
        <div className="space-y-2">
          <RefreshProgress progress={status.progress} />
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
          <div className="flex items-center gap-2">
            <RefreshButton />
            {refreshError && <span className="text-[11px] text-red-400">{refreshError}</span>}
          </div>
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
        <div className="flex flex-wrap items-center justify-between gap-3">
          <RoleSelector />
          <div
            title={
              "Only heroes picked at least 1.5x as often as the average hero — about 25 of 127, " +
              "or roughly two dozen per role. Leave it off to see the sharpest counter to this " +
              "lineup, which is often a hero nobody plays."
            }
          >
            <Toggle
              label="Meta picks only"
              checked={status.metaOnly}
              onChange={(v) => setMetaOnly(v)}
            />
          </div>
        </div>

        {/* Kept above the list rather than replacing it: a rebuild started
            mid-draft must not take the advice off screen. */}
        {status.refreshing && <RefreshProgress progress={status.progress} />}

        {advice.suggestions.length === 0 ? (
          <p className="text-sm text-muted">
            {draft.slots.some((s) => s.hero)
              ? "No suggestions for this role."
              : "Suggestions appear as heroes are identified."}
          </p>
        ) : (
          <>
            <AdviceList suggestions={advice.suggestions} position={status.position} />
            {/* The one sentence that makes the numbers readable. Without it
                a column of "+10.3" is just a number the user has to trust. */}
            <p className="text-[11px] text-muted">
              Each column is one hero already in the draft.{" "}
              <span className="text-green-400">+10.3</span> means this pick wins 10.3 points
              more often than its own average when that hero is on the other side;{" "}
              <span className="text-red-400">-2.6</span> means the reverse. Hover any number
              for the sample behind it. <span className="text-gold">Fit</span> combines those
              matchups with synergy and the hero&rsquo;s win rate in your role.
            </p>
          </>
        )}

        {advice.unresolved.length > 0 && (
          <p className="text-xs text-amber-400">
            Not in the dataset: {advice.unresolved.map(heroName).join(", ")} — the
            cache predates a patch, so these picks are missing from the advice.
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

        <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
          <p className="text-[11px] text-muted">
            {status.metaOnly ? "Meta picks only · " : ""}
            {status.heroCount} heroes · {status.bracket.replace("_", " + ")} ·{" "}
            <span title={`Dataset built ${built}`}>
              built {status.builtAt ? timeAgo(status.builtAt) : "unknown"}
            </span>{" "}
            · {advice.enemiesUsed} enemy and {advice.alliesUsed} ally picks counted
          </p>
          <RefreshButton />
          {refreshError && <span className="text-[11px] text-red-400">{refreshError}</span>}
        </div>
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
  }, [lineupSignature, stratz.ready, stratz.position, stratz.metaOnly, fetchAdvice]);

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
