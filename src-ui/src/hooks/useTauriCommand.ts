import { useState, useCallback } from 'react';
import { isTauri } from '../lib/tauri';

interface UseTauriCommandOptions<T> {
  /** Fallback value when not running in Tauri (for dev server) */
  mockFallback?: T;
}

interface UseTauriCommandResult<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  execute: (...args: unknown[]) => Promise<T | null>;
}

/**
 * Generic hook for calling Tauri IPC commands.
 * Falls back to mock data when running outside Tauri (standalone dev server).
 */
export function useTauriCommand<T>(
  command: string,
  options: UseTauriCommandOptions<T> = {}
): UseTauriCommandResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const execute = useCallback(
    async (...args: unknown[]): Promise<T | null> => {
      if (!isTauri()) {
        if (options.mockFallback !== undefined) {
          setData(options.mockFallback);
          return options.mockFallback;
        }
        setError('Not running in Tauri environment');
        return null;
      }

      setLoading(true);
      setError(null);

      try {
        const { invoke } = await import('@tauri-apps/api/core');
        const result = await invoke<T>(command, args[0] as Record<string, unknown> | undefined);
        setData(result);
        return result;
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        setError(message);
        return null;
      } finally {
        setLoading(false);
      }
    },
    [command, options.mockFallback]
  );

  return { data, loading, error, execute };
}
