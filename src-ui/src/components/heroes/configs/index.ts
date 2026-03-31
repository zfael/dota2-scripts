import type { HeroType } from "../../../types/game";
import type { ComponentType } from "react";

const configs: Record<HeroType, () => Promise<{ default: ComponentType }>> = {
  meepo: () => import("./MeepoConfig"),
  broodmother: () => import("./BroodmotherConfig"),
  huskar: () => import("./HuskarConfig"),
  largo: () => import("./LargoConfig"),
  legion_commander: () => import("./LegionCommanderConfig"),
  outworld_destroyer: () => import("./OutworldDestroyerConfig"),
  shadow_fiend: () => import("./ShadowFiendConfig"),
  tiny: () => import("./TinyConfig"),
};

export default configs;
