import { useUpdateStore } from "../../stores/updateStore";
import { Button } from "../common/Button";

export function UpdateBanner() {
  const updateState = useUpdateStore((s) => s.updateState);
  const applyUpdate = useUpdateStore((s) => s.applyUpdate);
  const dismissUpdate = useUpdateStore((s) => s.dismissUpdate);

  if (updateState.kind !== "available") return null;

  return (
    <div className="flex items-center justify-between gap-4 border-b border-border bg-elevated px-4 py-2">
      <div className="flex items-center gap-2">
        <span className="text-sm font-medium text-gold">
          🎉 Update v{updateState.version} available
        </span>
        {updateState.releaseNotes && (
          <span className="text-xs text-subtle">
            — {updateState.releaseNotes}
          </span>
        )}
      </div>
      <div className="flex items-center gap-2">
        <Button onClick={applyUpdate} className="h-7 px-3 text-xs">
          Apply Update
        </Button>
        <button
          type="button"
          onClick={dismissUpdate}
          className="text-xs text-subtle hover:text-content"
        >
          Dismiss
        </button>
      </div>
    </div>
  );
}
