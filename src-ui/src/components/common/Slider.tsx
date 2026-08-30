interface SliderProps {
  label: string;
  value: number;
  min: number;
  max: number;
  step?: number;
  onChange: (value: number) => void;
  suffix?: string;
  /** Explanatory line under the track. */
  hint?: string;
  disabled?: boolean;
}

/**
 * Label and monospaced readout on one baseline, then a native range input the
 * stylesheet paints as a hairline track with an accent bead (see global.css).
 */
export function Slider({
  label,
  value,
  min,
  max,
  step = 1,
  onChange,
  suffix = "",
  hint,
  disabled = false,
}: SliderProps) {
  return (
    <div>
      <div className="mb-1.5 flex items-baseline justify-between gap-3">
        <span className="text-sm text-subtle">{label}</span>
        <span className="font-mono text-xs text-content">
          {value}
          {suffix}
        </span>
      </div>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        disabled={disabled}
        aria-label={label}
        aria-valuemin={min}
        aria-valuemax={max}
        aria-valuenow={value}
        aria-valuetext={`${value}${suffix}`}
      />
      {hint && (
        <div className="mt-1.5 text-xs leading-relaxed text-muted">{hint}</div>
      )}
    </div>
  );
}
