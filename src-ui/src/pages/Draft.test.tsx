import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import Draft from "./Draft";
import { useConfigStore } from "../stores/configStore";
import { useDraftStore } from "../stores/draftStore";
import { useStratzStore } from "../stores/stratzStore";
import { mockConfig } from "../stores/mockData";
import { EMPTY_DRAFT_STATUS } from "../types/draft";
import { EMPTY_STRATZ_STATUS } from "../types/stratz";
import type { MatchupDetail, Suggestion } from "../types/stratz";

vi.mock("../lib/tauri", () => ({ isTauri: () => false }));

function detail(
  slug: string,
  displayName: string,
  offset: number,
  matches = 500,
): MatchupDetail {
  return { slug, displayName, offset, matches, contribution: offset * 0.9 };
}

function suggestion(overrides: Partial<Suggestion> = {}): Suggestion {
  return {
    slug: "marci",
    displayName: "Marci",
    score: 0.248,
    counter: 0.161,
    synergy: 0.058,
    positionWinRate: 0.569,
    pickRate: 0.037,
    bestAgainst: "Drow Ranger",
    vsEnemies: [
      detail("drow_ranger", "Drow Ranger", 0.103, 453),
      detail("vengefulspirit", "Vengeful Spirit", -0.026, 500),
    ],
    withAllies: [detail("obsidian_destroyer", "Outworld Destroyer", 0.042, 173)],
    counterSamples: 3107,
    ...overrides,
  };
}

describe("Draft advice panel", () => {
  beforeEach(() => {
    useConfigStore.setState({
      config: { ...mockConfig, draft: { ...mockConfig.draft, enabled: true } },
      loaded: true,
    });
    useDraftStore.setState({
      status: {
        ...EMPTY_DRAFT_STATUS,
        enabled: true,
        sessionId: "s1",
        slots: [
          { index: 0, isAlly: true, hero: "obsidian_destroyer", unknown: false, agreement: 1, bestScore: 0.9 },
          { index: 5, isAlly: false, hero: "necrolyte", unknown: false, agreement: 1, bestScore: 0.9 },
        ],
      },
      judged: {},
    });
    useStratzStore.setState({
      status: {
        ...EMPTY_STRATZ_STATUS,
        enabled: true,
        hasToken: true,
        ready: true,
        position: 2,
        heroCount: 127,
        bracket: "DIVINE_IMMORTAL",
      },
      advice: {
        suggestions: [suggestion(), suggestion({ slug: "lina", displayName: "Lina", score: 0.163 })],
        unresolved: [],
        alliesUsed: 1,
        enemiesUsed: 2,
      },
    });
  });

  it("names heroes the way players do, not by their internal slug", () => {
    render(<Draft />);
    // The Lineup panel used to title-case the slug, so the same hero read
    // "Necrolyte" here and "Necrophos" in the advice above it.
    expect(screen.getByText("Necrophos")).toBeInTheDocument();
    expect(screen.getByText("Outworld Destroyer")).toBeInTheDocument();
    expect(screen.queryByText("Necrolyte")).not.toBeInTheDocument();
    expect(screen.queryByText("Obsidian Destroyer")).not.toBeInTheDocument();
  });

  it("shows every matchup behind a pick, losing ones included", () => {
    render(<Draft />);
    // The good matchup and the bad one both appear as their own number, which
    // a single summed "vs +16.1" could never convey.
    expect(screen.getAllByText("+10.3").length).toBeGreaterThan(0);
    expect(screen.getAllByText("-2.6").length).toBeGreaterThan(0);
    expect(screen.getAllByText("+4.2").length).toBeGreaterThan(0);
  });

  it("reports win rate and popularity per suggestion", () => {
    render(<Draft />);
    expect(screen.getAllByText("56.9%").length).toBeGreaterThan(0);
    expect(screen.getAllByText("3.7%").length).toBeGreaterThan(0);
  });

  it("offers the meta filter and reflects it in the footer", () => {
    render(<Draft />);
    const toggle = screen.getByRole("switch", { name: /meta picks only/i });
    expect(toggle).toHaveAttribute("aria-checked", "false");

    useStratzStore.setState((s) => ({ status: { ...s.status, metaOnly: true } }));
    render(<Draft />);
    expect(screen.getAllByText(/Meta picks only ·/).length).toBeGreaterThan(0);
  });

  it("renders before any hero is identified, with no matchup columns", () => {
    // The first advice arrives with an empty lineup. A grid template built as
    // `repeat(0, ...)` is invalid CSS and would collapse the whole table.
    useStratzStore.setState((s) => ({
      advice: {
        ...s.advice,
        suggestions: [suggestion({ vsEnemies: [], withAllies: [] })],
        alliesUsed: 0,
        enemiesUsed: 0,
      },
    }));
    render(<Draft />);
    const row = screen.getByText("Marci").closest("div")!;
    expect(row.style.gridTemplateColumns).not.toContain("repeat(0");
    expect(row.style.gridTemplateColumns).toContain("minmax(7rem, 1fr)");
  });

  it("offers a manual rebuild and keeps the advice on screen during one", () => {
    render(<Draft />);
    expect(screen.getByRole("button", { name: "Refresh now" })).toBeEnabled();

    useStratzStore.setState((s) => ({ status: { ...s.status, refreshing: true, progress: 40 } }));
    render(<Draft />);
    // The old panel replaced everything with a progress card, which took the
    // advice away for a minute at the exact moment it is wanted.
    expect(screen.getAllByText("Marci").length).toBeGreaterThan(0);
    expect(screen.getAllByText(/Rebuilding the matchup dataset/).length).toBeGreaterThan(0);
    for (const button of screen.getAllByRole("button", { name: "Refreshing…" })) {
      expect(button).toBeDisabled();
    }
  });

  it("marks a hero with no popularity data rather than calling it unpopular", () => {
    useStratzStore.setState((s) => ({
      advice: { ...s.advice, suggestions: [suggestion({ pickRate: null, positionWinRate: null })] },
    }));
    render(<Draft />);
    // Unknown win rate and unknown pick rate both read as "—" rather than as
    // 0%, which would look like a hero nobody plays and nobody wins with.
    expect(screen.getAllByText("—").length).toBeGreaterThanOrEqual(2);
    expect(
      screen.getByTitle(/Popularity unknown — STRATZ never returned/),
    ).toBeInTheDocument();
  });
});
