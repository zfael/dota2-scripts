import { Field } from "./Field";

interface NumberInputProps {
  label: string;
  value: number;
  onChange: (value: number) => void;
  min?: number;
  max?: number;
  /** Arrow-key increment. Also what the browser validates against, so fractional
   * values need this set or they read as invalid. */
  step?: number;
  suffix?: string;
  hint?: string;
  disabled?: boolean;
}

export function NumberInput({
  label,
  value,
  onChange,
  min,
  max,
  step,
  suffix,
  hint,
  disabled = false,
}: NumberInputProps) {
  return (
    <Field label={label} hint={hint}>
      <div className="relative flex items-center">
        <input
          type="number"
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          min={min}
          max={max}
          step={step}
          disabled={disabled}
          aria-label={label}
          className={`h-9 w-full rounded-md border border-border bg-input px-3 font-mono text-sm
                     text-content transition-colors hover:border-border-strong
                     focus:border-accent focus:outline-none
                     disabled:cursor-not-allowed disabled:bg-elevated disabled:text-muted
                     ${suffix ? "pr-10" : ""}`}
        />
        {suffix && (
          <span className="pointer-events-none absolute right-3 font-mono text-xs text-muted">
            {suffix}
          </span>
        )}
      </div>
    </Field>
  );
}
