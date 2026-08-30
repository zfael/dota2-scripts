export type AlertTone = "neutral" | "accent" | "success" | "warning" | "danger" | "info";

interface AlertProps {
  children: React.ReactNode;
  tone?: AlertTone;
  title?: string;
  className?: string;
}

const tones: Record<AlertTone, { box: string; mark: string }> = {
  neutral: { box: "bg-elevated border-border", mark: "bg-muted" },
  accent: { box: "bg-accent-soft border-transparent", mark: "bg-accent" },
  success: { box: "bg-success-soft border-transparent", mark: "bg-success" },
  warning: { box: "bg-warning-soft border-transparent", mark: "bg-warning" },
  danger: { box: "bg-danger-soft border-transparent", mark: "bg-danger" },
  info: { box: "bg-info-soft border-transparent", mark: "bg-info" },
};

/**
 * The design system's inline notice: a coloured rail, an optional bold line,
 * then body copy. Replaces the ad-hoc "⚠ …" paragraphs the pages used before.
 */
export function Alert({ children, tone = "neutral", title, className = "" }: AlertProps) {
  const t = tones[tone];
  return (
    <div
      className={`flex gap-3 rounded-md border px-4 py-3 text-sm text-subtle ${t.box} ${className}`}
    >
      <span className={`w-1.5 shrink-0 self-stretch rounded-full ${t.mark}`} />
      <div className="flex flex-1 flex-col gap-0.5 leading-relaxed">
        {title && <span className="text-sm font-semibold text-content">{title}</span>}
        <div>{children}</div>
      </div>
    </div>
  );
}
