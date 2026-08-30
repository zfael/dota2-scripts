import { NavLink } from "react-router-dom";
import { NAV_GROUPS } from "../../lib/nav";
import { HEROES } from "../../types/game";
import { useUIStore } from "../../stores/uiStore";

/** Only Heroes carries a count; everything else would be noise. */
const TRAILING: Record<string, string> = {
  "/heroes": String(HEROES.length),
};

export function Sidebar() {
  const sidebarCollapsed = useUIStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useUIStore((s) => s.toggleSidebar);
  const appVersion = useUIStore((s) => s.appVersion);

  return (
    <aside
      className={`flex h-full shrink-0 flex-col gap-3 border-r border-border bg-sunken p-3 transition-all duration-200 ${
        sidebarCollapsed ? "w-16" : "w-[236px]"
      }`}
    >
      <div
        className={`flex items-center gap-2 px-2 py-1.5 ${
          sidebarCollapsed ? "justify-center px-0" : ""
        }`}
      >
        <span className="grid h-[26px] w-[26px] shrink-0 place-items-center rounded-sm bg-gold text-sm font-bold text-accent-fg">
          D2
        </span>
        {!sidebarCollapsed && (
          <span className="text-sm font-semibold tracking-[-0.01em] text-content">
            D2 Scripts
          </span>
        )}
      </div>

      <nav className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto">
        {NAV_GROUPS.map((group) => (
          <div key={group.label} className="flex flex-col gap-px">
            {!sidebarCollapsed && (
              <span className="px-2 pt-2 pb-1 font-mono text-2xs tracking-[0.06em] text-muted uppercase">
                {group.label}
              </span>
            )}
            {group.items.map(({ to, label, icon: Icon }) => (
              <NavLink
                key={to}
                to={to}
                end={to === "/"}
                title={sidebarCollapsed ? label : undefined}
                className={({ isActive }) =>
                  `flex items-center gap-2 rounded-sm p-2 text-sm font-medium transition-colors ${
                    sidebarCollapsed ? "justify-center" : ""
                  } ${
                    isActive
                      ? "bg-accent-soft text-accent-text"
                      : "text-subtle hover:bg-elevated hover:text-content"
                  }`
                }
              >
                <Icon className="h-[18px] w-[18px] shrink-0 opacity-90" strokeWidth={1.5} />
                {!sidebarCollapsed && (
                  <>
                    <span className="truncate">{label}</span>
                    {TRAILING[to] && (
                      <span className="ml-auto font-mono text-2xs text-muted">
                        {TRAILING[to]}
                      </span>
                    )}
                  </>
                )}
              </NavLink>
            ))}
          </div>
        ))}
      </nav>

      <div
        className={`flex shrink-0 items-center gap-2 border-t border-border p-2 font-mono text-2xs text-muted ${
          sidebarCollapsed ? "justify-center" : "justify-between"
        }`}
      >
        {!sidebarCollapsed && <span>v{appVersion}</span>}
        <button
          type="button"
          onClick={toggleSidebar}
          title={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          className="grid h-[26px] w-[26px] cursor-pointer place-items-center rounded-sm border border-border text-xs text-muted transition-colors hover:bg-elevated hover:text-content"
        >
          {sidebarCollapsed ? "»" : "«"}
        </button>
      </div>
    </aside>
  );
}
