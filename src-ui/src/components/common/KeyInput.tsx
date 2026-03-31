import { useState } from "react";

interface KeyInputProps {
  label: string;
  value: string;
  onChange: (key: string) => void;
  disabled?: boolean;
}

export function KeyInput({ label, value, onChange, disabled = false }: KeyInputProps) {
  const [listening, setListening] = useState(false);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    if (listening) {
      onChange(e.key.length === 1 ? e.key.toUpperCase() : e.key);
      setListening(false);
    }
  };

  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <button
        type="button"
        disabled={disabled}
        onClick={() => setListening(true)}
        onKeyDown={handleKeyDown}
        onBlur={() => setListening(false)}
        className={`
          flex h-8 w-full items-center rounded-md border px-3 font-mono text-sm
          transition-colors
          ${listening
            ? "border-border-accent bg-elevated text-gold animate-pulse"
            : "border-border bg-input text-content"
          }
          ${disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"}
        `}
      >
        {listening ? "Press a key..." : value || "—"}
      </button>
    </div>
  );
}
