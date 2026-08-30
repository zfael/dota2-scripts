interface DangerBadgeProps {
  text?: string;
}

export function DangerBadge({ text = "⚠ DANGER" }: DangerBadgeProps) {
  return (
    <span className="inline-flex h-5 animate-pulse items-center rounded-full bg-danger px-2 text-2xs font-semibold text-white">
      {text}
    </span>
  );
}
