import { NavLink } from "react-router-dom";
import {
  LayoutDashboard,
  Swords,
  Shield,
  CircleDot,
  Axe,
  ScrollText,
  Activity,
  Settings,
} from "lucide-react";

const navItems = [
  { to: "/", label: "Dashboard", icon: LayoutDashboard },
  { to: "/heroes", label: "Heroes", icon: Swords },
  { to: "/danger", label: "Danger", icon: Shield },
  { to: "/soul-ring", label: "Soul Ring", icon: CircleDot },
  { to: "/armlet", label: "Armlet", icon: Axe },
  { to: "/activity", label: "Activity", icon: ScrollText },
  { to: "/diagnostics", label: "Diagnostics", icon: Activity },
  { to: "/settings", label: "Settings", icon: Settings },
];

export function Sidebar() {
  return (
    <aside className="flex h-full w-[200px] shrink-0 flex-col border-r border-border bg-base">
      <div className="p-4">
        <h1 className="text-lg font-semibold text-gold">D2 Scripts</h1>
      </div>
      <nav className="flex-1 space-y-0.5 px-2">
        {navItems.map(({ to, label, icon: Icon }) => (
          <NavLink
            key={to}
            to={to}
            end={to === "/"}
            className={({ isActive }) =>
              `flex items-center gap-3 rounded-md px-3 py-2.5 text-sm transition-colors ${
                isActive
                  ? "border-l-[3px] border-gold bg-elevated text-gold"
                  : "border-l-[3px] border-transparent text-subtle hover:bg-elevated hover:text-content"
              }`
            }
          >
            <Icon className="h-5 w-5 shrink-0" />
            <span>{label}</span>
          </NavLink>
        ))}
      </nav>
      <div className="border-t border-border p-4">
        <span className="text-xs text-muted">v0.1.0-dev</span>
      </div>
    </aside>
  );
}
