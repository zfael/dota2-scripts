import { Toggle } from "./Toggle";

interface SettingRowProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}

/**
 * The design's most repeated shape: a titled (optionally described) setting on
 * the left, its switch pinned right. Using one component for it keeps the
 * baseline of the switch aligned with the label across every page.
 */
export function SettingRow({
  label,
  description,
  checked,
  onChange,
  disabled = false,
}: SettingRowProps) {
  return (
    <div className="flex items-start justify-between gap-4">
      <div className="min-w-0">
        <div className="font-medium text-content">{label}</div>
        {description && (
          <div className="text-xs leading-relaxed text-muted">{description}</div>
        )}
      </div>
      <Toggle
        label=""
        ariaLabel={label}
        checked={checked}
        onChange={onChange}
        disabled={disabled}
      />
    </div>
  );
}
