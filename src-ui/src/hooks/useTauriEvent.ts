import { useEffect, useRef } from 'react';
import { isTauri } from '../lib/tauri';

/**
 * Generic hook for subscribing to Tauri events.
 * Automatically cleans up subscription on unmount.
 * No-op when not running in Tauri.
 */
export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void
): void {
  const handlerRef = useRef(handler);
  handlerRef.current = handler;

  useEffect(() => {
    if (!isTauri()) return;

    let unlisten: (() => void) | undefined;
    let cancelled = false;

    const setup = async () => {
      const { listen } = await import('@tauri-apps/api/event');
      if (cancelled) return;
      unlisten = await listen<T>(eventName, (event) => {
        handlerRef.current(event.payload);
      });
      if (cancelled) {
        unlisten();
      }
    };

    setup();

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [eventName]);
}
