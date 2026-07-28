import type { LanePath, MapPoint, WaveSnapshot, WaveConfidence } from "../../types/waves";

/**
 * Presentational vector map. Pure — all data arrives as props so it can be reused
 * unchanged by the minimap overlay window.
 */
interface WaveMapProps {
  lanePaths: LanePath[];
  snapshot: WaveSnapshot | null;
  /** Hides the base markers and legend for the small overlay rendering. */
  compact?: boolean;
  /**
   * Draw the lane polylines and river. Worth turning off over Dota's minimap,
   * which draws both already; the in-app panel has nothing underneath it, so the
   * dots would float in empty space without them.
   */
  showLanes?: boolean;
}

const VIEWBOX = 100;

/**
 * Normalised map space has its origin at the bottom-left; SVG's is at the top-left,
 * so the y-axis flips here and nowhere else.
 */
function toSvg(point: MapPoint): { x: number; y: number } {
  return { x: point.x * VIEWBOX, y: (1 - point.y) * VIEWBOX };
}

function toPolyline(points: MapPoint[]): string {
  return points
    .map((point) => {
      const { x, y } = toSvg(point);
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

/**
 * Predictions are estimates, so they fade as the model's assumptions stop holding.
 * Never fully opaque — these are never measurements.
 */
const CONFIDENCE_OPACITY: Record<WaveConfidence, number> = {
  High: 0.95,
  Degrading: 0.6,
  Low: 0.35,
};

const RADIANT_COLOR = "#2ECC71";
const DIRE_COLOR = "#E74C3C";

export function WaveMap({
  lanePaths,
  snapshot,
  compact = false,
  showLanes = true,
}: WaveMapProps) {
  const opacity = snapshot ? CONFIDENCE_OPACITY[snapshot.confidence] : 0;

  return (
    <svg
      viewBox={`0 0 ${VIEWBOX} ${VIEWBOX}`}
      className="h-full w-full"
      role="img"
      aria-label="Predicted creep wave positions"
    >
      {!compact && (
        <rect
          x={0}
          y={0}
          width={VIEWBOX}
          height={VIEWBOX}
          rx={3}
          fill="#0A0E14"
          stroke="#2A3040"
          strokeWidth={0.6}
        />
      )}

      {showLanes && (
        <>
          {/* River, drawn as the anti-diagonal between the two bases. */}
          <line
            x1={0}
            y1={0}
            x2={VIEWBOX}
            y2={VIEWBOX}
            stroke="#3498DB"
            strokeWidth={0.8}
            strokeOpacity={0.18}
          />

          {/* Lane paths — the same polylines the model interpolates along. */}
          {lanePaths.map((path) => (
            <polyline
              key={path.lane}
              points={toPolyline(path.points)}
              fill="none"
              stroke="#2A3040"
              strokeWidth={1.6}
              strokeLinecap="round"
              strokeLinejoin="round"
              data-testid={`lane-path-${path.lane}`}
            />
          ))}
        </>
      )}

      {!compact && (
        <>
          <circle cx={10} cy={90} r={4} fill={RADIANT_COLOR} fillOpacity={0.2} stroke={RADIANT_COLOR} strokeWidth={0.7} />
          <circle cx={90} cy={10} r={4} fill={DIRE_COLOR} fillOpacity={0.2} stroke={DIRE_COLOR} strokeWidth={0.7} />
        </>
      )}

      {/* Clash markers: where the current wave pair meets. */}
      {snapshot?.clashes.map((clash) => {
        const { x, y } = toSvg(clash.point);
        return (
          <g key={`clash-${clash.lane}`} opacity={opacity}>
            <circle
              cx={x}
              cy={y}
              r={2.6}
              fill="none"
              stroke="#C8AA6E"
              strokeWidth={0.7}
              strokeDasharray="1.2 1"
              data-testid={`clash-${clash.lane}`}
            />
          </g>
        );
      })}

      {/* Wave dots. Soft outer halo keeps them reading as estimates, not fixes. */}
      {snapshot?.waves.map((wave) => {
        const { x, y } = toSvg(wave.point);
        const color = wave.team === "Radiant" ? RADIANT_COLOR : DIRE_COLOR;
        return (
          <g key={`${wave.lane}-${wave.team}`} opacity={opacity}>
            <circle cx={x} cy={y} r={3.4} fill={color} fillOpacity={0.18} />
            <circle
              cx={x}
              cy={y}
              r={1.7}
              fill={color}
              stroke={wave.hasClashed ? "#C8AA6E" : "none"}
              strokeWidth={wave.hasClashed ? 0.6 : 0}
              data-testid={`wave-${wave.lane}-${wave.team}`}
            />
          </g>
        );
      })}
    </svg>
  );
}
