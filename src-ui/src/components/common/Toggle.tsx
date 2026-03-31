interface ToggleProps {
  label?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

export function Toggle({ label, checked, onChange, disabled = false }: ToggleProps) {
  return (
    <label className="flex items-center justify-between gap-3 cursor-pointer select-none">
      {label && <span className="text-sm text-subtle">{label}</span>}
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => !disabled && onChange(!checked)}
        className={`
          relative inline-flex h-5 w-9 shrink-0 rounded-full transition-colors duration-200
          ${checked ? "bg-gold" : "bg-input"}
          ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}
        `}
      >
        <span
          className={`
            pointer-events-none inline-block h-4 w-4 rounded-full shadow-sm
            transform transition-transform duration-200 mt-0.5
            ${checked ? "translate-x-4 bg-white" : "translate-x-0.5 bg-muted"}
          `}
        />
      </button>
    </label>
  );
}
