interface ManaBarProps {
  percent: number;
  size?: "sm" | "md";
  /** Topbar variant: a 6px rail with the readout set beside it in mono. */
  thin?: boolean;
}

export function ManaBar({ percent, size = "sm", thin = false }: ManaBarProps) {
  const clamped = Math.max(0, Math.min(100, percent));

  if (thin) {
    return (
      <div className="flex items-center gap-2">
        <div className="h-1.5 flex-1 overflow-hidden rounded-full bg-raised">
          <div
            data-fill
            className="h-full rounded-full bg-info transition-all duration-300"
            style={{ width: `${clamped}%` }}
          />
        </div>
        <span className="font-mono text-2xs text-muted">{percent}%</span>
      </div>
    );
  }

  const h = size === "sm" ? "h-2" : "h-4";
  return (
    <div className={`relative w-full ${h} overflow-hidden rounded-full bg-raised`}>
      <div
        data-fill
        className={`absolute top-0 left-0 ${h} rounded-full bg-info transition-all duration-300`}
        style={{ width: `${clamped}%` }}
      />
      <span className="absolute inset-0 flex items-center justify-center font-mono text-[10px] font-medium text-content drop-shadow">
        {percent}%
      </span>
    </div>
  );
}
