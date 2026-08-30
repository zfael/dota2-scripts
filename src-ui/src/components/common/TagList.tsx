import { useState } from "react";
import { Plus, X } from "lucide-react";

interface TagListProps {
  label: string;
  items: string[];
  onChange: (items: string[]) => void;
  disabled?: boolean;
}

export function TagList({ label, items, onChange, disabled = false }: TagListProps) {
  const [adding, setAdding] = useState(false);
  const [draft, setDraft] = useState("");

  const remove = (index: number) => {
    onChange(items.filter((_, i) => i !== index));
  };

  const add = () => {
    const trimmed = draft.trim();
    if (trimmed && !items.includes(trimmed)) {
      onChange([...items, trimmed]);
    }
    setDraft("");
    setAdding(false);
  };

  return (
    <div className="flex flex-col gap-2">
      <span className="text-xs font-medium text-subtle">{label}</span>
      <div className="flex flex-wrap gap-1.5">
        {items.map((item, i) => (
          <span
            key={item}
            className="inline-flex h-5 items-center gap-1 rounded-full border border-border-strong px-2 text-2xs text-subtle"
          >
            {item}
            {!disabled && (
              <button
                type="button"
                onClick={() => remove(i)}
                aria-label={`remove ${item}`}
                className="-mr-0.5 cursor-pointer rounded-full text-muted hover:text-danger-text"
              >
                <X className="h-3 w-3" />
              </button>
            )}
          </span>
        ))}
        {!disabled &&
          (adding ? (
            <input
              autoFocus
              placeholder="Add item..."
              value={draft}
              onChange={(e) => setDraft(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && add()}
              onBlur={() => {
                add();
                setAdding(false);
              }}
              className="h-5 w-32 rounded-full border border-dashed border-border bg-input px-2 text-2xs text-content focus:outline-none"
            />
          ) : (
            <button
              type="button"
              onClick={() => setAdding(true)}
              aria-label="add"
              className="inline-flex h-5 cursor-pointer items-center gap-1 rounded-full border border-dashed border-border px-2 text-2xs text-muted hover:text-content"
            >
              <Plus className="h-3 w-3" /> Add
            </button>
          ))}
      </div>
    </div>
  );
}
