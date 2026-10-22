/**
 * AppearanceContext
 *
 * Holds the shell-level appearance state that used to live in the demo's
 * AppShell: the three-state theme, the §14 accessibility overrides (Reduce
 * Motion / Reduce Transparency), and the sidebar collapsed flag.
 *
 * State persists to localStorage (`uk.*`). Fresh installs start in the light
 * reference theme; users can choose System or Dark in Settings.
 * A single effect stamps `data-theme` / `data-rm` / `data-rt` onto
 * <html>; tokens.css and the Tailwind `dark:` variant both key off those.
 *
 * Note: `data-theme="system"` is never written — attribute *absence* is
 * the contract for "System" in both tokens.css and the `dark:` variant.
 */

import React, { createContext, useContext, useEffect, useMemo, useState } from 'react';
import { useMediaQuery } from '@/components/ui/primitives';

export type Theme = 'system' | 'light' | 'dark';

interface AppearanceCtx {
  theme: Theme;
  setTheme: (t: Theme) => void;
  reduceMotion: boolean;
  setReduceMotion: (v: boolean) => void;
  reduceTransparency: boolean;
  setReduceTransparency: (v: boolean) => void;
  collapsed: boolean;
  setCollapsed: (v: boolean | ((c: boolean) => boolean)) => void;
}

const Ctx = createContext<AppearanceCtx | null>(null);

/** Read a stored boolean override, or fall back to the system default. */
function storedFlag(key: string, systemDefault: boolean): boolean {
  try {
    const raw = localStorage.getItem(key);
    if (raw === 'true') return true;
    if (raw === 'false') return false;
  } catch { /* localStorage unavailable — use system default */ }
  return systemDefault;
}

export const AppearanceProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const systemReduceMotion = useMediaQuery('(prefers-reduced-motion: reduce)');
  const systemReduceTransparency = useMediaQuery('(prefers-reduced-transparency: reduce)');

  const [theme, setTheme] = useState<Theme>(() => {
    try { return (localStorage.getItem('uk.theme') as Theme) || 'light'; }
    catch { return 'light'; }
  });
  const [reduceMotion, setReduceMotion] = useState(() => storedFlag('uk.reduceMotion', systemReduceMotion));
  const [reduceTransparency, setReduceTransparency] = useState(() =>
    storedFlag('uk.reduceTransparency', systemReduceTransparency),
  );
  const [collapsed, setCollapsed] = useState(() => storedFlag('uk.sidebarCollapsed', false));

  // §14 — apply theme + accessibility overrides to the document.
  useEffect(() => {
    const root = document.documentElement;
    if (theme === 'system') root.removeAttribute('data-theme');
    else root.setAttribute('data-theme', theme);
    root.toggleAttribute('data-rm', reduceMotion);
    root.toggleAttribute('data-rt', reduceTransparency);
  }, [theme, reduceMotion, reduceTransparency]);

  // Persist any explicit choices.
  useEffect(() => { try { localStorage.setItem('uk.theme', theme); } catch {} }, [theme]);
  useEffect(() => { try { localStorage.setItem('uk.reduceMotion', String(reduceMotion)); } catch {} }, [reduceMotion]);
  useEffect(() => { try { localStorage.setItem('uk.reduceTransparency', String(reduceTransparency)); } catch {} }, [reduceTransparency]);
  useEffect(() => { try { localStorage.setItem('uk.sidebarCollapsed', String(collapsed)); } catch {} }, [collapsed]);

  const value = useMemo<AppearanceCtx>(
    () => ({ theme, setTheme, reduceMotion, setReduceMotion, reduceTransparency, setReduceTransparency, collapsed, setCollapsed }),
    [theme, reduceMotion, reduceTransparency, collapsed],
  );

  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
};

export const useAppearance = () => {
  const ctx = useContext(Ctx);
  if (!ctx) throw new Error('useAppearance must be used within AppearanceProvider');
  return ctx;
};
