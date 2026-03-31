import { Link } from "react-router-dom";
import { ArrowLeft } from "lucide-react";
import type { HeroInfo } from "../../types/game";

interface HeroPageProps {
  hero: HeroInfo;
  children: React.ReactNode;
}

export function HeroPage({ hero, children }: HeroPageProps) {
  return (
    <div className="space-y-6 p-6">
      <div className="flex items-center gap-4">
        <Link
          to="/heroes"
          className="flex items-center gap-1 text-sm text-subtle hover:text-content"
        >
          <ArrowLeft className="h-4 w-4" /> Heroes
        </Link>
        <span className="text-muted">/</span>
        <div className="flex items-center gap-2">
          <span className="text-2xl">{hero.icon}</span>
          <h2 className="text-xl font-semibold">{hero.displayName}</h2>
        </div>
      </div>
      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">{children}</div>
    </div>
  );
}
