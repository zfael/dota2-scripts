import { Link } from "react-router-dom";
import { Avatar } from "../common/Avatar";
import { Button } from "../common/Button";
import type { HeroInfo } from "../../types/game";

interface HeroPageProps {
  hero: HeroInfo;
  children: React.ReactNode;
}

export function HeroPage({ hero, children }: HeroPageProps) {
  return (
    <div className="space-y-4 p-6">
      <div className="flex items-center gap-3">
        <Link to="/heroes">
          <Button variant="ghost" size="sm">
            Heroes
          </Button>
        </Link>
        <span className="text-muted">/</span>
        <Avatar name={hero.displayName} glyph={hero.icon} size="sm" />
        <h2 className="text-lg font-semibold text-content">{hero.displayName}</h2>
        <span className="font-mono text-2xs text-muted">{hero.internalName}</span>
      </div>
      <div className="grid grid-cols-1 gap-4 lg:grid-cols-2">{children}</div>
    </div>
  );
}
