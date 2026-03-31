import { useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { useUpdateStore } from "../../stores/updateStore";
import { Button } from "../common/Button";

function ReleaseNotesModal({
  version,
  notes,
  onClose,
  onApply,
}: {
  version: string;
  notes: string;
  onClose: () => void;
  onApply: () => void;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={onClose}
    >
      <div
        className="mx-4 flex max-h-[80vh] w-full max-w-2xl flex-col rounded-lg border border-border bg-elevated shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between border-b border-border px-6 py-4">
          <h2 className="text-lg font-semibold text-content">
            Release Notes — v{version}
          </h2>
          <button
            type="button"
            onClick={onClose}
            className="text-subtle hover:text-content"
          >
            ✕
          </button>
        </div>
        <div className="flex-1 overflow-y-auto px-6 py-4">
          <div className="prose prose-invert prose-sm max-w-none prose-headings:text-gold prose-a:text-gold prose-strong:text-content prose-code:rounded prose-code:bg-base prose-code:px-1.5 prose-code:py-0.5 prose-code:text-content prose-pre:bg-base prose-li:text-subtle">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{notes}</ReactMarkdown>
          </div>
        </div>
        <div className="flex items-center justify-end gap-3 border-t border-border px-6 py-4">
          <button
            type="button"
            onClick={onClose}
            className="rounded px-4 py-2 text-sm text-subtle hover:text-content"
          >
            Close
          </button>
          <Button onClick={onApply} className="px-4 py-2 text-sm">
            Apply Update
          </Button>
        </div>
      </div>
    </div>
  );
}

export function UpdateBanner() {
  const updateState = useUpdateStore((s) => s.updateState);
  const applyUpdate = useUpdateStore((s) => s.applyUpdate);
  const dismissUpdate = useUpdateStore((s) => s.dismissUpdate);
  const [showNotes, setShowNotes] = useState(false);

  if (updateState.kind !== "available") return null;

  return (
    <>
      <div className="flex items-center justify-between gap-4 border-b border-border bg-elevated px-4 py-2">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gold">
            🎉 Update v{updateState.version} available
          </span>
          {updateState.releaseNotes && (
            <button
              type="button"
              onClick={() => setShowNotes(true)}
              className="text-xs text-subtle underline decoration-dotted hover:text-content"
            >
              View Release Notes
            </button>
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
      {showNotes && updateState.releaseNotes && (
        <ReleaseNotesModal
          version={updateState.version}
          notes={updateState.releaseNotes}
          onClose={() => setShowNotes(false)}
          onApply={() => {
            setShowNotes(false);
            applyUpdate();
          }}
        />
      )}
    </>
  );
}
