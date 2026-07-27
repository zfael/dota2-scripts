import { describe, it, expect } from "vitest";
import { render } from "@testing-library/react";
import { WaveMap } from "./WaveMap";
import type { LanePath, WaveSnapshot } from "../../types/waves";

const lanePaths: LanePath[] = [
  {
    lane: "Mid",
    points: [
      { x: 0.17, y: 0.17 },
      { x: 0.83, y: 0.83 },
    ],
  },
];

function snapshot(overrides: Partial<WaveSnapshot> = {}): WaveSnapshot {
  return {
    enabled: true,
    clockTimeSeconds: 10,
    nextSpawnTimeSeconds: 30,
    secondsUntilNextSpawn: 20,
    currentWaveAgeSeconds: 10,
    confidence: "High",
    waves: [
      {
        lane: "Mid",
        team: "Radiant",
        progress: 0.29,
        point: { x: 0.36, y: 0.36 },
        hasClashed: false,
      },
      {
        lane: "Mid",
        team: "Dire",
        progress: 0.71,
        point: { x: 0.64, y: 0.64 },
        hasClashed: false,
      },
    ],
    clashes: [
      {
        lane: "Mid",
        progress: 0.5,
        point: { x: 0.5, y: 0.5 },
        secondsUntilClash: 7,
      },
    ],
    ...overrides,
  };
}

describe("WaveMap", () => {
  it("renders lane paths even with no snapshot", () => {
    const { container } = render(<WaveMap lanePaths={lanePaths} snapshot={null} />);

    expect(container.querySelector("polyline")).not.toBeNull();
    expect(container.querySelector('[data-testid="wave-Mid-Radiant"]')).toBeNull();
  });

  it("renders a dot per wave and a marker per clash", () => {
    const { container } = render(
      <WaveMap lanePaths={lanePaths} snapshot={snapshot()} />,
    );

    expect(container.querySelector('[data-testid="wave-Mid-Radiant"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="wave-Mid-Dire"]')).not.toBeNull();
    expect(container.querySelector('[data-testid="clash-Mid"]')).not.toBeNull();
  });

  it("flips the y-axis from map space into SVG space", () => {
    const { container } = render(
      <WaveMap lanePaths={lanePaths} snapshot={snapshot()} />,
    );

    // Map y=0.36 (near the Radiant/bottom corner) must land low on screen,
    // which in SVG means a high cy.
    const radiant = container.querySelector('[data-testid="wave-Mid-Radiant"]');
    expect(Number(radiant?.getAttribute("cy"))).toBeCloseTo(64, 1);
    expect(Number(radiant?.getAttribute("cx"))).toBeCloseTo(36, 1);
  });

  it("fades the prediction as confidence decays", () => {
    const high = render(
      <WaveMap lanePaths={lanePaths} snapshot={snapshot({ confidence: "High" })} />,
    );
    const low = render(
      <WaveMap lanePaths={lanePaths} snapshot={snapshot({ confidence: "Low" })} />,
    );

    const opacityOf = (result: ReturnType<typeof render>) =>
      Number(
        result.container
          .querySelector('[data-testid="wave-Mid-Radiant"]')
          ?.closest("g")
          ?.getAttribute("opacity"),
      );

    expect(opacityOf(low)).toBeLessThan(opacityOf(high));
  });

  it("marks clashed waves distinctly", () => {
    const clashed = snapshot({
      waves: [
        {
          lane: "Mid",
          team: "Radiant",
          progress: 0.5,
          point: { x: 0.5, y: 0.5 },
          hasClashed: true,
        },
      ],
    });

    const { container } = render(<WaveMap lanePaths={lanePaths} snapshot={clashed} />);
    const dot = container.querySelector('[data-testid="wave-Mid-Radiant"]');

    expect(dot?.getAttribute("stroke")).toBe("#C8AA6E");
  });

  it("omits base markers in compact mode for the overlay", () => {
    const full = render(<WaveMap lanePaths={lanePaths} snapshot={null} />);
    const compact = render(<WaveMap lanePaths={lanePaths} snapshot={null} compact />);

    const circlesIn = (result: ReturnType<typeof render>) =>
      result.container.querySelectorAll("circle").length;

    expect(circlesIn(compact)).toBeLessThan(circlesIn(full));
    expect(compact.container.querySelector("rect")).toBeNull();
  });
});
