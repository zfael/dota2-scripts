export type BadgeTone =
  | "neutral"
  | "accent"
  | "success"
  | "warning"
  | "danger"
  | "info";

interface BadgeProps {
  children: React.ReactNode;
  tone?: BadgeTone;
  /** Leading status dot, tinted with the badge's own text colour. */
  dot?: boolean;
  /** Transparent fill with a hairline border — used for key caps and item lists. */
  outline?: boolean;
  /** Squared-off and monospaced. The design uses this for single keys. */
  square?: boolean;
  className?: string;
  title?: string;
}

const fills: Record<BadgeTone, string> = {
  neutral: "bg-raised",
  accent: "bg-accent-soft",
  success: "bg-success-soft",
  warning: "bg-warning-soft",
  danger: "bg-danger-soft",
  info: "bg-info-soft",
};

const texts: Record<BadgeTone, string> = {
  neutral: "text-subtle",
  accent: "text-accent-text",
  success: "text-success-text",
  warning: "text-warning-text",
  danger: "text-danger-text",
  info: "text-info-text",
};

export function Badge({
  children,
  tone = "neutral",
  dot = false,
  outline = false,
  square = false,
  className = "",
  title,
}: BadgeProps) {
  const shape = square ? "rounded-xs font-mono" : "rounded-full";
  const fill = outline ? "border border-border-strong" : fills[tone];

  return (
    <span
      title={title}
      className={`inline-flex h-5 shrink-0 items-center gap-1 px-2 text-2xs font-medium whitespace-nowrap ${shape} ${fill} ${texts[tone]} ${className}`}
    >
      {dot && <span className="h-[5px] w-[5px] shrink-0 rounded-full bg-current" />}
      {children}
    </span>
  );
}
