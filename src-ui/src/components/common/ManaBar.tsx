interface ManaBarProps {
  percent: number;
  size?: "sm" | "md";
}

export function ManaBar({ percent, size = "sm" }: ManaBarProps) {
  const h = size === "sm" ? "h-2" : "h-4";
  return (
    <div className={`relative w-full ${h} overflow-hidden rounded-full bg-input`}>
      <div
        data-fill
        className={`absolute left-0 top-0 ${h} rounded-full bg-info transition-all duration-300`}
        style={{ width: `${Math.max(0, Math.min(100, percent))}%` }}
      />
      <span className="absolute inset-0 flex items-center justify-center text-[10px] font-mono font-medium text-content drop-shadow">
        {percent}%
      </span>
    </div>
  );
}
