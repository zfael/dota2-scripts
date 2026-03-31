interface DangerBadgeProps {
  text?: string;
}

export function DangerBadge({ text = "⚠ DANGER" }: DangerBadgeProps) {
  return (
    <span className="inline-flex items-center rounded bg-danger px-2 py-0.5 text-xs font-semibold text-white animate-pulse">
      {text}
    </span>
  );
}
