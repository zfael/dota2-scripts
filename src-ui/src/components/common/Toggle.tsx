interface ToggleProps {
  label?: string;
  /** Overrides the accessible name when the visible label is empty or decorative. */
  ariaLabel?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Toggle({
  label,
  ariaLabel,
  checked,
  onChange,
  disabled = false,
}: ToggleProps) {
  const name = ariaLabel ?? (label || undefined);

  return (
    <label className="flex cursor-pointer items-center justify-between gap-4 select-none">
      {label && <span className="text-sm text-content">{label}</span>}
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={name}
        disabled={disabled}
        onClick={() => !disabled && onChange(!checked)}
        className={`
          relative inline-flex h-[22px] w-[38px] shrink-0 rounded-full border
          transition-colors duration-200
          ${checked ? "border-accent bg-gold" : "border-border bg-raised"}
          ${disabled ? "cursor-not-allowed opacity-45" : "cursor-pointer"}
        `}
      >
        <span
          className={`
            pointer-events-none absolute top-[2px] left-[2px] inline-block h-4 w-4
            rounded-full shadow-sm transition-transform duration-200
            ${checked ? "translate-x-4 bg-accent-fg" : "translate-x-0 bg-subtle"}
          `}
        />
      </button>
    </label>
  );
}
