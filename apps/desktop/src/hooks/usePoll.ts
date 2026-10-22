/**
 * usePoll — generic polling hook for any async fetcher.
 *
 * Returns [data, loading, error, refetch].
 */
import { useCallback, useEffect, useRef, useState } from 'react';

export function usePoll<T>(fetcher: () => Promise<T>, intervalMs: number) {
  const [data, setData]     = useState<T | null>(null);
  const [loading, setLoad]  = useState(true);
  const [error, setError]   = useState<string | null>(null);
  const mounted = useRef(true);

  const run = useCallback(async () => {
    try {
      setLoad(true);
      const d = await fetcher();
      if (mounted.current) { setData(d); setError(null); }
    } catch (e: any) {
      if (mounted.current) setError(e?.message ?? 'unknown error');
    } finally {
      if (mounted.current) setLoad(false);
    }
  }, [fetcher]);

  useEffect(() => {
    mounted.current = true;
    run();
    const id = setInterval(run, intervalMs);
    return () => { mounted.current = false; clearInterval(id); };
  }, [run, intervalMs]);

  return { data, loading, error, refetch: run } as const;
}
