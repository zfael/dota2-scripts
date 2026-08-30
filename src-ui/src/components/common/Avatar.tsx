export type AvatarSize = "xs" | "sm" | "md" | "lg" | "xl";

interface AvatarProps {
  /** Falls back to initials when no glyph is supplied. */
  name: string;
  /** Hero emoji. The design draws initials; the app already identifies heroes
   * by glyph everywhere else, so the glyph wins when there is one. */
  glyph?: string;
  size?: AvatarSize;
  status?: "online" | "away" | "offline";
  className?: string;
}

const sizes: Record<AvatarSize, string> = {
  xs: "h-[22px] w-[22px] text-[9px]",
  sm: "h-[26px] w-[26px] text-2xs",
  md: "h-8 w-8 text-xs",
  lg: "h-11 w-11 text-base",
  xl: "h-16 w-16 text-xl",
};

const statusColors = {
  online: "bg-success",
  away: "bg-warning",
  offline: "bg-muted",
};

function initials(name: string): string {
  return name
    .split(/\s+/)
    .slice(0, 2)
    .map((word) => word[0] ?? "")
    .join("")
    .toUpperCase();
}

export function Avatar({
  name,
  glyph,
  size = "md",
  status,
  className = "",
}: AvatarProps) {
  return (
    <span
      className={`relative inline-flex shrink-0 select-none items-center justify-center overflow-hidden rounded-full bg-accent-soft font-semibold text-accent-text ${sizes[size]} ${className}`}
      aria-hidden={!!glyph}
      title={name}
    >
      {glyph ?? initials(name)}
      {status && (
        <span
          className={`absolute -right-px -bottom-px h-[9px] w-[9px] rounded-full border-2 border-surface ${statusColors[status]}`}
        />
      )}
    </span>
  );
}
