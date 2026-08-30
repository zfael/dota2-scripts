import { Link } from "react-router-dom";
import { Avatar } from "../components/common/Avatar";
import { Badge } from "../components/common/Badge";
import { HEROES } from "../types/game";
import { useGameStore } from "../stores/gameStore";

export default function Heroes() {
  const heroName = useGameStore((s) => s.game.heroName);

  return (
    <div className="space-y-4 p-6">
      <p className="text-subtle">
        Select a hero to view and configure its automation settings.
      </p>
      <div className="grid grid-cols-3 gap-3 sm:grid-cols-4 lg:grid-cols-5">
        {HEROES.map((hero) => {
          const isActive =
            heroName?.toLowerCase() === hero.displayName.toLowerCase();
          return (
            <Link
              key={hero.id}
              to={`/heroes/${hero.id}`}
              className={`flex flex-col items-center gap-2 rounded-lg border bg-surface px-3 py-4 transition-colors hover:bg-elevated ${
                isActive ? "border-accent" : "border-border"
              }`}
            >
              <Avatar name={hero.displayName} glyph={hero.icon} size="lg" />
              <span className="text-center text-sm font-medium text-content">
                {hero.displayName}
              </span>
              <span className="text-2xs text-muted">{hero.role}</span>
              {isActive && <Badge tone="accent">Active</Badge>}
            </Link>
          );
        })}
      </div>
    </div>
  );
}
