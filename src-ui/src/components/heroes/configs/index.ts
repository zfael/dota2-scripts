import type { HeroType } from "../../../types/game";
import type { ComponentType } from "react";

const configs: Record<HeroType, () => Promise<{ default: ComponentType }>> = {
  meepo: () => import("./MeepoConfig"),
  broodmother: () => import("./BroodmotherConfig"),
  earth_spirit: () => import("./EarthSpiritConfig"),
  ember_spirit: () => import("./EmberSpiritConfig"),
  huskar: () => import("./HuskarConfig"),
  invoker: () => import("./InvokerConfig"),
  largo: () => import("./LargoConfig"),
  legion_commander: () => import("./LegionCommanderConfig"),
  magnus: () => import("./MagnusConfig"),
  mirana: () => import("./MiranaConfig"),
  outworld_destroyer: () => import("./OutworldDestroyerConfig"),
  shadow_fiend: () => import("./ShadowFiendConfig"),
  slark: () => import("./SlarkConfig"),
  snapfire: () => import("./SnapfireConfig"),
  tiny: () => import("./TinyConfig"),
};

export default configs;
