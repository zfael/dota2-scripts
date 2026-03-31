import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

interface CardProps {
  title: string;
  children: React.ReactNode;
  collapsible?: boolean;
  defaultOpen?: boolean;
  className?: string;
}

export function Card({
  title,
  children,
  collapsible = false,
  defaultOpen = true,
  className = "",
}: CardProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div
      className={`rounded-lg border border-border bg-surface p-4 ${className}`}
    >
      <button
        type="button"
        onClick={() => collapsible && setOpen(!open)}
        className={`flex w-full items-center justify-between text-left ${
          collapsible ? "cursor-pointer" : "cursor-default"
        }`}
      >
        <h3 className="text-sm font-semibold text-content">{title}</h3>
        {collapsible &&
          (open ? (
            <ChevronDown className="h-4 w-4 text-subtle" />
          ) : (
            <ChevronRight className="h-4 w-4 text-subtle" />
          ))}
      </button>
      <div
        className={`mt-3 space-y-3 ${open ? "" : "hidden"}`}
        aria-hidden={!open}
      >
        {children}
      </div>
    </div>
  );
}
