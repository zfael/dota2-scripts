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
  disabled = false,
}: NumberInputProps) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <div className="flex items-center gap-2">
        <input
          type="number"
          value={value}
          onChange={(e) => onChange(Number(e.target.value))}
          min={min}
          max={max}
          step={step}
          disabled={disabled}
          className="h-8 w-full rounded-md border border-border bg-input px-3 font-mono text-sm
                     text-content focus:border-border-accent focus:outline-none
                     disabled:cursor-not-allowed disabled:opacity-50"
        />
        {suffix && <span className="text-xs text-subtle shrink-0">{suffix}</span>}
      </div>
    </div>
  );
}
