interface HPBarProps {
  percent: number;
  size?: "sm" | "md";
}

function hpColor(pct: number): string {
  if (pct > 60) return "bg-success";
  if (pct > 30) return "bg-warning";
  return "bg-danger";
}

export function HPBar({ percent, size = "sm" }: HPBarProps) {
  const h = size === "sm" ? "h-2" : "h-4";
  return (
    <div className={`relative w-full ${h} overflow-hidden rounded-full bg-input`}>
      <div
        data-fill
        className={`absolute left-0 top-0 ${h} rounded-full transition-all duration-300 ${hpColor(percent)}`}
        style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
      />
      <span className="absolute inset-0 flex items-center justify-center text-[10px] font-mono font-medium text-content drop-shadow">
        {percent}%
      </span>
    </div>
  );
}
