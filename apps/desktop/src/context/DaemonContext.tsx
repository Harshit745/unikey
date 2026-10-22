/**
 * DaemonContext
 *
 * On mount (when running inside Tauri), calls `daemon_start` to extract
 * and launch the embedded daemon. Then polls /health on a 2-second cadence.
 * Exposes `health`, `version`, and `daemonReachable` to every component.
 * `retryConnect()` re-polls and, if offline, re-attempts daemon_start.
 */

import React, { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';
import { api } from '@/api/client';
import type { HealthStatus, VersionInfo } from '@/api/types';

/** Detect the Tauri runtime (only present inside the webview). */
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function invokeDaemonStart(): Promise<void> {
  if (!isTauri) return;
  try {
    // Dynamic import keeps the Vite dev server working when Tauri isn't present.
    const { invoke } = await import('@tauri-apps/api/core');
    await invoke('daemon_start');
  } catch {
    // Non-fatal — the daemon might already be running externally.
  }
}

interface DaemonCtx {
  health: HealthStatus;
  version: VersionInfo | null;
  daemonReachable: boolean;
  retryConnect: () => void;
}

const Ctx = createContext<DaemonCtx>({
  health: { status: 'down' },
  version: null,
  daemonReachable: false,
  retryConnect: () => {},
});

export const useDaemon = () => useContext(Ctx);

const POLL_MS = 2000;

export const DaemonProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [health, setHealth]   = useState<HealthStatus>({ status: 'down' });
  const [version, setVersion] = useState<VersionInfo | null>(null);
  const mounted = useRef(true);
  const fetchedVersion = useRef(false);

  const poll = useCallback(async () => {
    try {
      const h = await api.health();
      if (!mounted.current) return;
      setHealth(h);
      if (h.status === 'ok' && !fetchedVersion.current) {
        fetchedVersion.current = true;
        api.version().then(v => { if (mounted.current) setVersion(v); }).catch(() => {});
      }
    } catch {
      if (mounted.current) setHealth({ status: 'down' });
    }
  }, []);

  useEffect(() => {
    mounted.current = true;
    // Try to start the daemon when running inside Tauri.
    invokeDaemonStart().finally(() => poll());
    const id = setInterval(poll, POLL_MS);
    return () => { mounted.current = false; clearInterval(id); };
  }, [poll]);

  const retryConnect = useCallback(() => {
    void invokeDaemonStart().finally(() => poll());
  }, [poll]);

  return (
    <Ctx.Provider value={{ health, version, daemonReachable: health.status === 'ok', retryConnect }}>
      {children}
    </Ctx.Provider>
  );
};
