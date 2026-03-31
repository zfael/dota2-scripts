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
    <div className="space-y-2">
      <label className="text-xs text-subtle">{label}</label>
      <div className="flex flex-wrap gap-2">
        {items.map((item, i) => (
          <span
            key={item}
            className="inline-flex items-center gap-1 rounded-full border border-border bg-elevated px-2.5 py-0.5 text-xs text-content"
          >
            {item}
            {!disabled && (
              <button
                type="button"
                onClick={() => remove(i)}
                aria-label={`remove ${item}`}
                className="ml-0.5 rounded-full p-0.5 text-muted hover:text-danger"
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
              onBlur={() => { add(); setAdding(false); }}
              className="h-6 w-28 rounded-full border border-dashed border-border bg-input px-2 text-xs text-content focus:outline-none"
            />
          ) : (
            <button
              type="button"
              onClick={() => setAdding(true)}
              aria-label="add"
              className="inline-flex items-center gap-1 rounded-full border border-dashed border-border px-2.5 py-0.5 text-xs text-muted hover:text-content"
            >
              <Plus className="h-3 w-3" /> Add
            </button>
          ))}
      </div>
    </div>
  );
}
