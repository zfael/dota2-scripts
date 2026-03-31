import { useParams, Navigate } from "react-router-dom";
import { Suspense, lazy, useMemo } from "react";
import { HEROES, type HeroType } from "../types/game";
import { HeroPage } from "../components/heroes/HeroPage";
import configs from "../components/heroes/configs";

export default function HeroDetail() {
  const { heroId } = useParams<{ heroId: string }>();
  const hero = HEROES.find((h) => h.id === heroId);

  const ConfigComponent = useMemo(() => {
    if (!heroId || !(heroId in configs)) return null;
    return lazy(configs[heroId as HeroType]);
  }, [heroId]);

  if (!hero || !ConfigComponent) {
    return <Navigate to="/heroes" replace />;
  }

  return (
    <HeroPage hero={hero}>
      <Suspense
        fallback={
          <p className="col-span-2 text-subtle">Loading config...</p>
        }
      >
        <ConfigComponent />
      </Suspense>
    </HeroPage>
  );
}

