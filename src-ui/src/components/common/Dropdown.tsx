import { ChevronDown } from "lucide-react";

interface DropdownProps {
  label: string;
  value: string;
  options: { value: string; label: string }[];
  onChange: (value: string) => void;
  disabled?: boolean;
}

export function Dropdown({
  label,
  value,
  options,
  onChange,
  disabled = false,
}: DropdownProps) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <div className="relative">
        <select
          value={value}
          onChange={(e) => onChange(e.target.value)}
          disabled={disabled}
          className="h-8 w-full appearance-none rounded-md border border-border bg-input px-3 pr-8
                     font-mono text-sm text-content focus:border-border-accent focus:outline-none
                     disabled:cursor-not-allowed disabled:opacity-50"
        >
          {options.map((opt) => (
            <option key={opt.value} value={opt.value}>
              {opt.label}
            </option>
          ))}
        </select>
        <ChevronDown className="pointer-events-none absolute right-2 top-2 h-4 w-4 text-muted" />
      </div>
    </div>
  );
}
