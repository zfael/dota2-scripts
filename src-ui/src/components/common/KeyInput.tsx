import { useState } from "react";
import { Field } from "./Field";

interface KeyInputProps {
  label: string;
  value: string;
  onChange: (key: string) => void;
  hint?: string;
  disabled?: boolean;
}

export function KeyInput({
  label,
  value,
  onChange,
  hint,
  disabled = false,
}: KeyInputProps) {
  const [listening, setListening] = useState(false);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    e.preventDefault();
    if (listening) {
      onChange(e.key.length === 1 ? e.key.toUpperCase() : e.key);
      setListening(false);
    }
  };

  return (
    <Field label={label} hint={hint}>
      <button
        type="button"
        disabled={disabled}
        onClick={() => setListening(true)}
        onKeyDown={handleKeyDown}
        onBlur={() => setListening(false)}
        className={`
          flex h-9 w-full items-center rounded-md border px-3 font-mono text-sm
          transition-colors
          ${
            listening
              ? "animate-pulse border-accent bg-elevated text-accent-text"
              : "border-border bg-input text-content hover:border-border-strong"
          }
          ${disabled ? "cursor-not-allowed opacity-45" : "cursor-pointer"}
        `}
      >
        {listening ? "Press a key..." : value || "—"}
      </button>
    </Field>
  );
}
