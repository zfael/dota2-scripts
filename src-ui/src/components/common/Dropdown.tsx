import { ChevronDown } from "lucide-react";
import { Field } from "./Field";

interface DropdownProps {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  hint?: string;
  disabled?: boolean;
}

export function Dropdown({
  label,
  value,
  options,
  onChange,
  hint,
  disabled = false,
}: DropdownProps) {
  return (
    <Field label={label} hint={hint}>
      <div className="relative">
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          aria-label={label}
          className="h-9 w-full appearance-none rounded-md border border-border bg-input px-3 pr-9
                     text-sm text-content transition-colors hover:border-border-strong
                     focus:border-accent focus:outline-none
                     disabled:cursor-not-allowed disabled:bg-elevated disabled:text-muted"
        >
          {options.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <ChevronDown className="pointer-events-none absolute top-2.5 right-3 h-4 w-4 text-muted" />
      </div>
    </Field>
  );
}
