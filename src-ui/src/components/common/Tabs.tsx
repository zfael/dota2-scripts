interface TabsProps<T extends string> {
  items: { label: string; value: T }[];
  value: T;
  onChange: (value: T) => void;
  className?: string;
}

/**
 * The design system's pill tabs: a sunken track with the selected pill raised
 * onto the card surface.
 */
export function Tabs<T extends string>({
  items,
  value,
  onChange,
  className = "",
}: TabsProps<T>) {
  return (
    <div
      role="tablist"
      className={`inline-flex items-center gap-0.5 rounded-md bg-elevated p-[3px] ${className}`}
    >
      {items.map((item) => {
        const selected = item.value === value;
        return (
          <button
            key={item.value}
            type="button"
            role="tab"
            aria-selected={selected}
            onClick={() => onChange(item.value)}
            className={`cursor-pointer rounded-sm px-3 py-1.5 text-sm font-medium transition-colors ${
              selected
                ? "bg-surface text-content shadow-sm"
                : "text-muted hover:text-content"
            }`}
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
