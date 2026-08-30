interface FieldProps {
  label: string;
  hint?: string;
  error?: string;
  htmlFor?: string;
  children: React.ReactNode;
  className?: string;
}

/**
 * Label / control / hint stack. Every input on a config page goes through this
 * so the label type, gap and hint colour stay identical across pages.
 */
export function Field({
  label,
  hint,
  error,
  htmlFor,
  children,
  className = "",
}: FieldProps) {
  return (
    <div className={`flex flex-col gap-1.5 ${className}`}>
      <label htmlFor={htmlFor} className="text-xs font-medium text-subtle">
        {label}
      </label>
      {children}
      {hint && <span className="text-xs leading-relaxed text-muted">{hint}</span>}
      {error && <span className="text-xs text-danger-text">{error}</span>}
    </div>
  );
}

export function Divider({ className = "" }: { className?: string }) {
  return <div className={`h-px w-full bg-border ${className}`} />;
}
