import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

interface CardProps {
  title?: string;
  /** Secondary line under the title — the design leans on this to explain what
   * a card owns instead of repeating it as body copy. */
  subtitle?: string;
  /** Rendered on the right of the header (a badge, a small button). */
  action?: React.ReactNode;
  /** Rendered in a tinted, bordered strip under the body. */
  footer?: React.ReactNode;
  children: React.ReactNode;
  collapsible?: boolean;
  defaultOpen?: boolean;
  className?: string;
  /** Drops the body's default vertical rhythm for content that lays itself out. */
  flushBody?: boolean;
}

export function Card({
  title,
  subtitle,
  action,
  footer,
  children,
  collapsible = false,
  defaultOpen = true,
  className = "",
  flushBody = false,
}: CardProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div
      className={`flex flex-col overflow-hidden rounded-lg border border-border bg-surface ${className}`}
    >
      {(title || action) && (
        <div className="flex items-start justify-between gap-3 px-5 pt-4 pb-4">
          <button
            type="button"
            onClick={() => collapsible && setOpen(!open)}
            className={`flex min-w-0 flex-1 items-start justify-between gap-3 text-left ${
              collapsible ? "cursor-pointer" : "cursor-default"
            }`}
          >
            <span className="min-w-0">
              <span className="block text-base font-semibold tracking-[-0.01em] text-content">
                {title}
              </span>
              {subtitle && (
                <span className="mt-[3px] block text-xs leading-relaxed text-muted">
                  {subtitle}
                </span>
              )}
            </span>
            {collapsible &&
              (open ? (
                <ChevronDown className="mt-0.5 h-4 w-4 shrink-0 text-muted" />
              ) : (
                <ChevronRight className="mt-0.5 h-4 w-4 shrink-0 text-muted" />
              ))}
          </button>
          {action && <div className="shrink-0">{action}</div>}
        </div>
      )}
      <div
        className={`px-5 pb-5 text-sm text-subtle ${title ? "" : "pt-5"} ${
          flushBody ? "" : "space-y-4"
        } ${open ? "" : "hidden"}`}
        aria-hidden={!open}
      >
        {children}
      </div>
      {footer && (
        <div className="flex items-center gap-2 border-t border-border bg-elevated px-5 py-3">
          {footer}
        </div>
      )}
    </div>
  );
}
