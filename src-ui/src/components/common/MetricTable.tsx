export interface Metric {
  label: string;
  value: string | number;
}

/**
 * The compact two-column table the design uses for every diagnostics readout:
 * a metric name on the left, a right-aligned monospaced value on the right.
 */
export function MetricTable({ rows }: { rows: Metric[] }) {
  return (
    <div className="overflow-hidden rounded-lg border border-border">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr>
            <th className="border-b border-border bg-elevated px-3 py-1.5 text-left text-2xs font-semibold tracking-[0.06em] text-muted uppercase">
              Metric
            </th>
            <th className="border-b border-border bg-elevated px-3 py-1.5 text-right text-2xs font-semibold tracking-[0.06em] text-muted uppercase">
              Value
            </th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.label} className="last:[&>td]:border-b-0">
              <td className="border-b border-border px-3 py-1.5 text-subtle">
                {row.label}
              </td>
              <td className="border-b border-border px-3 py-1.5 text-right font-mono text-xs text-content">
                {row.value}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
