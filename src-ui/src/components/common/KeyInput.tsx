import { useState } from "react";

interface KeyInputProps {
  label: string;
  value: string;
  onChange: (key: string) => void;
  disabled?: boolean;
}

/** Keys that are only ever pressed *with* another key. Capturing one alone is
 *  always the user still reaching for the real key, never their choice. */
const MODIFIER_KEYS = new Set(["Shift", "Control", "Alt", "Meta", "AltGraph", "CapsLock"]);

/**
 * Translate a DOM `KeyboardEvent.key` into the name the Rust side stores.
 *
 * The two sides must agree on spelling or the binding round-trips into
 * `KeyBinding::Unknown` and silently does nothing — see `src/input/binding.rs`,
 * which owns the canonical list.
 */
function keyNameFromEvent(event: React.KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;

  // The space bar arrives as a single space. Sending that verbatim is what made
  // a Space binding show up as an empty button.
  if (event.key === " ") return "Space";

  switch (event.key) {
    case "ArrowUp":
      return "Up";
    case "ArrowDown":
      return "Down";
    case "ArrowLeft":
      return "Left";
    case "ArrowRight":
      return "Right";
    case "Escape":
      return "Escape";
    default:
      break;
  }

  // Numpad digits report location 3, and are a different physical key from the
  // number row even though `event.key` is the same character.
  if (event.location === 3 && /^[0-9]$/.test(event.key)) {
    return `Numpad${event.key}`;
  }

  // Single characters go down lowercased. Uppercasing here used to make enigo
  // synthesize Shift+key, which in Dota queues an ability instead of casting it.
  return event.key.length === 1 ? event.key.toLowerCase() : event.key;
}

/** DOM `MouseEvent.button` -> the binding names Rust understands. */
const MOUSE_BUTTON_NAMES: Record<number, string> = {
  1: "Mouse3",
  3: "Mouse4",
  4: "Mouse5",
};

export function KeyInput({ label, value, onChange, disabled = false }: KeyInputProps) {
  const [listening, setListening] = useState(false);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (!listening) return;
    e.preventDefault();

    const key = keyNameFromEvent(e);
    // A modifier on its own leaves the capture armed, so the user can finish
    // reaching for the key they actually meant.
    if (key === null) return;

    onChange(key);
    setListening(false);
  };

  // Mouse buttons fire no keydown at all, so without this a Mouse4 bind was
  // impossible to enter no matter what the backend accepted.
  const handleMouseDown = (e: React.MouseEvent) => {
    if (!listening) return;

    const name = MOUSE_BUTTON_NAMES[e.button];
    // Left and right are deliberately not bindable: Dota needs them for move
    // and attack orders, and grabbing one would break the game.
    if (!name) return;

    e.preventDefault();
    e.stopPropagation();
    onChange(name);
    setListening(false);
  };

  return (
    <div className="space-y-1">
      <label className="text-xs text-subtle">{label}</label>
      <button
        type="button"
        disabled={disabled}
        onClick={() => setListening(true)}
        onKeyDown={handleKeyDown}
        onMouseDown={handleMouseDown}
        onAuxClick={(e) => e.preventDefault()}
        onContextMenu={(e) => listening && e.preventDefault()}
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
        {listening ? "Press a key or mouse button..." : value || "—"}
      </button>
    </div>
  );
}
