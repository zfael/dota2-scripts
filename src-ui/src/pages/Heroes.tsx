import { Link } from "react-router-dom";
import { HEROES } from "../types/game";
import { useGameStore } from "../stores/gameStore";

export default function Heroes() {
  const heroName = useGameStore((s) => s.game.heroName);

  return (
    <div className="space-y-6 p-6">
      <h2 className="text-xl font-semibold">Heroes</h2>
      <p className="text-sm text-subtle">
        Select a hero to view and configure its automation settings.
      </p>
      <div className="grid grid-cols-2 gap-4 sm:grid-cols-4">
        {HEROES.map((hero) => {
          const isActive =
            heroName?.toLowerCase() === hero.displayName.toLowerCase();
          return (
            <Link
              key={hero.id}
              to={`/heroes/${hero.id}`}
              className={`flex flex-col items-center gap-3 rounded-lg border p-4 transition-colors hover:bg-elevated ${
                isActive
                  ? "border-gold bg-elevated"
                  : "border-border bg-surface"
              }`}
            >
              <span className="flex h-14 w-14 items-center justify-center rounded-full bg-base text-2xl">
                {hero.icon}
              </span>
              <div className="text-center">
                <p className="text-sm font-medium text-content">
                  {hero.displayName}
                </p>
                <p className="text-xs text-muted">{hero.role}</p>
              </div>
              {isActive && (
                <span className="rounded-full bg-gold/20 px-2 py-0.5 text-[10px] font-medium text-gold">
                  Active
                </span>
              )}
            </Link>
          );
        })}
      </div>
    </div>
  );
}
