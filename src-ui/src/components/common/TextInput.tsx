interface TextInputProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  disabled?: boolean;
}

export function TextInput({
  label,
  value,
  onChange,
  placeholder,
  disabled = false,
}: TextInputProps) {
  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <input
        type="text"
        value={value}
        placeholder={placeholder}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        className="h-8 w-full rounded-md border border-border bg-input px-3
                   font-mono text-sm text-content placeholder:text-muted
                   focus:border-border-accent focus:outline-none
                   disabled:cursor-not-allowed disabled:opacity-50"
      />
    </div>
  );
}
