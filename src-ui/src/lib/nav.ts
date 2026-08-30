import {
  Activity,
  Axe,
  Bell,
  CircleDot,
  Footprints,
  HeartPulse,
  LayoutDashboard,
  Radar,
  ScrollText,
  Settings,
  Shield,
  Swords,
  Waves,
  type LucideIcon,
} from "lucide-react";

export interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
}

export interface NavGroup {
  label: string;
  items: NavItem[];
}

/**
 * The redesign splits the flat nav into four groups so the sidebar reads as
 * "what is happening / what it does / what it sees / the app itself".
 */
export const NAV_GROUPS: NavGroup[] = [
  {
    label: "Live",
    items: [
      { to: "/", label: "Dashboard", icon: LayoutDashboard },
      { to: "/heroes", label: "Heroes", icon: Swords },
    ],
  },
  {
    label: "Automation",
    items: [
      { to: "/survivability", label: "Survivability", icon: HeartPulse },
      { to: "/danger", label: "Danger", icon: Shield },
      { to: "/soul-ring", label: "Soul Ring", icon: CircleDot },
      { to: "/armlet", label: "Armlet", icon: Axe },
      { to: "/boots", label: "Boots", icon: Footprints },
    ],
  },
  {
    label: "Intel",
    items: [
      { to: "/minimap", label: "Minimap", icon: Radar },
      { to: "/waves", label: "Waves", icon: Waves },
      { to: "/alerts", label: "Alerts", icon: Bell },
    ],
  },
  {
    label: "System",
    items: [
      { to: "/activity", label: "Activity", icon: ScrollText },
      { to: "/diagnostics", label: "Diagnostics", icon: Activity },
      { to: "/settings", label: "Settings", icon: Settings },
    ],
  },
];

/** Route → topbar title. The redesign moved page titles out of the pages. */
const PAGE_TITLES: [RegExp, string][] = [
  [/^\/$/, "Dashboard"],
  [/^\/heroes\/.+/, "Hero Config"],
  [/^\/heroes\/?$/, "Heroes"],
  [/^\/survivability/, "Survivability"],
  [/^\/danger/, "Danger Detection"],
  [/^\/soul-ring/, "Soul Ring"],
  [/^\/armlet/, "Armlet"],
  [/^\/boots/, "Boots"],
  [/^\/minimap/, "Minimap Intelligence"],
  [/^\/waves/, "Wave Tracker"],
  [/^\/alerts/, "Objective Alerts"],
  [/^\/activity/, "Activity Log"],
  [/^\/diagnostics/, "Diagnostics"],
  [/^\/settings/, "Settings"],
];

export function pageTitleFor(pathname: string): string {
  return PAGE_TITLES.find(([pattern]) => pattern.test(pathname))?.[1] ?? "D2 Scripts";
}
