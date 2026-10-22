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
const RESTART_BACKOFF_MS = 5000;

export const DaemonProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [health, setHealth]   = useState<HealthStatus>({ status: 'down' });
  const [version, setVersion] = useState<VersionInfo | null>(null);
  const mounted = useRef(true);
  const fetchedVersion = useRef(false);
  const startInFlight = useRef(false);
  const lastStartAttempt = useRef(0);

  const ensureDaemon = useCallback(async (force = false) => {
    if (!isTauri || startInFlight.current) return;

    const now = Date.now();
    if (!force && now - lastStartAttempt.current < RESTART_BACKOFF_MS) return;

    lastStartAttempt.current = now;
    startInFlight.current = true;
    try {
      await invokeDaemonStart();
    } finally {
      startInFlight.current = false;
    }
  }, []);

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
      if (mounted.current) {
        setHealth({ status: 'down' });
        // A packaged app owns its daemon. Recover automatically after a
        // crash, but rate-limit attempts so a persistent failure stays quiet.
        void ensureDaemon();
      }
    }
  }, [ensureDaemon]);

  useEffect(() => {
    mounted.current = true;
    // Try to start the daemon when running inside Tauri.
    ensureDaemon(true).finally(() => poll());
    const id = setInterval(poll, POLL_MS);
    return () => { mounted.current = false; clearInterval(id); };
  }, [ensureDaemon, poll]);

  const retryConnect = useCallback(() => {
    void ensureDaemon(true).finally(() => poll());
  }, [ensureDaemon, poll]);

  return (
    <Ctx.Provider value={{ health, version, daemonReachable: health.status === 'ok', retryConnect }}>
      {children}
    </Ctx.Provider>
  );
};
