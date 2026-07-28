import type { LanePath, MapPoint, WaveSnapshot, WaveConfidence } from "../../types/waves";

/**
 * Where normalised map space sits inside the rendered box.
 *
 * The in-app panel is a bare square, so map space fills it and the identity below
 * applies. The overlay window is not: it covers Dota's whole minimap *panel*,
 * whose bezel and corner buttons frame a map texture that is inset and not
 * centred. Painting map space straight onto that window puts the lanes a few
 * percent out — small in absolute terms, but a wave dot only has to miss by a
 * couple of pixels to sit in the trees instead of the lane.
 *
 * Offsets are fractions of the box, positive right and *down* (screen
 * convention, not map convention); scales are fractions applied about the centre.
 */
export interface MapCalibration {
  offsetX: number;
  offsetY: number;
  scaleX: number;
  scaleY: number;
}

export const IDENTITY_CALIBRATION: MapCalibration = {
  offsetX: 0,
  offsetY: 0,
  scaleX: 1,
  scaleY: 1,
};

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
  /** Defaults to the identity — only the overlay needs anything else. */
  calibration?: MapCalibration;
  /**
   * Outline map space and mark its centre. Alignment is far easier to judge
   * against a box with a known meaning than against lane lines that are
   * themselves only approximate.
   */
  showBounds?: boolean;
}

const VIEWBOX = 100;

/**
 * Map space to SVG space.
 *
 * Normalised map space has its origin at the bottom-left; SVG's is at the top-left,
 * so the y-axis flips here and nowhere else. The calibration is applied in the same
 * step, about the centre of the box.
 */
function toSvg(point: MapPoint, calibration: MapCalibration): { x: number; y: number } {
  const { offsetX, offsetY, scaleX, scaleY } = calibration;
  return {
    x: (0.5 + offsetX + (point.x - 0.5) * scaleX) * VIEWBOX,
    y: (0.5 + offsetY + (0.5 - point.y) * scaleY) * VIEWBOX,
  };
}

function toPolyline(points: MapPoint[], calibration: MapCalibration): string {
  return points
    .map((point) => {
      const { x, y } = toSvg(point, calibration);
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
  calibration = IDENTITY_CALIBRATION,
  showBounds = false,
}: WaveMapProps) {
  const opacity = snapshot ? CONFIDENCE_OPACITY[snapshot.confidence] : 0;
  const project = (point: MapPoint) => toSvg(point, calibration);

  // Map space corners, in SVG order. Kept as points rather than a rect so the
  // calibration is applied by the same code path as everything else.
  const topLeft = project({ x: 0, y: 1 });
  const bottomRight = project({ x: 1, y: 0 });

  return (
    // `preserveAspectRatio="none"` on purpose: the default letterboxes a square
    // viewBox inside the overlay's non-square window, which would silently
    // override the vertical half of the calibration below.
    <svg
      viewBox={`0 0 ${VIEWBOX} ${VIEWBOX}`}
      preserveAspectRatio="none"
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
            x1={topLeft.x}
            y1={topLeft.y}
            x2={bottomRight.x}
            y2={bottomRight.y}
            stroke="#3498DB"
            strokeWidth={0.8}
            strokeOpacity={0.18}
          />

          {/* Lane paths — the same polylines the model interpolates along. */}
          {lanePaths.map((path) => (
            <polyline
              key={path.lane}
              points={toPolyline(path.points, calibration)}
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

      {/* Calibration guides: align this box to the minimap's map area. */}
      {showBounds && (
        <g data-testid="map-bounds">
          <rect
            x={topLeft.x}
            y={topLeft.y}
            width={bottomRight.x - topLeft.x}
            height={bottomRight.y - topLeft.y}
            fill="none"
            stroke="#C8AA6E"
            strokeWidth={0.6}
            strokeDasharray="2 1.5"
          />
          <line
            x1={(topLeft.x + bottomRight.x) / 2}
            y1={topLeft.y}
            x2={(topLeft.x + bottomRight.x) / 2}
            y2={bottomRight.y}
            stroke="#C8AA6E"
            strokeWidth={0.35}
            strokeOpacity={0.5}
          />
          <line
            x1={topLeft.x}
            y1={(topLeft.y + bottomRight.y) / 2}
            x2={bottomRight.x}
            y2={(topLeft.y + bottomRight.y) / 2}
            stroke="#C8AA6E"
            strokeWidth={0.35}
            strokeOpacity={0.5}
          />
        </g>
      )}

      {!compact &&
        [
          { point: { x: 0.1, y: 0.1 }, color: RADIANT_COLOR },
          { point: { x: 0.9, y: 0.9 }, color: DIRE_COLOR },
        ].map(({ point, color }) => {
          const { x, y } = project(point);
          return (
            <circle
              key={color}
              cx={x}
              cy={y}
              r={4}
              fill={color}
              fillOpacity={0.2}
              stroke={color}
              strokeWidth={0.7}
            />
          );
        })}

      {/* Clash markers: where the current wave pair meets. */}
      {snapshot?.clashes.map((clash) => {
        const { x, y } = project(clash.point);
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
        const { x, y } = project(wave.point);
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
