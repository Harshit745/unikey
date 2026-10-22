/**
 * TopBar.tsx
 * ----------
 * Window header: breadcrumb + page title, a glass search field, the live
 * daemon status chip, the system/light/dark picker, and the host badge.
 * Ported from the app-ui study; the status chip is wired to the real
 * daemon health poll instead of a hardcoded "3 connected".
 */

import React, { useEffect, useRef } from 'react';
import { useLocation } from 'react-router-dom';
import { motion } from 'motion/react';
import { Search, ChevronRight, Lock } from 'lucide-react';
import { useAppearance, type Theme } from '@/context/AppearanceContext';
import { useDaemon } from '@/context/DaemonContext';
import { Segmented } from '@/components/ui/primitives';

export const ROUTE_META: Record<string, { title: string; subtitle: string }> = {
  '/': { title: 'Dashboard', subtitle: 'System overview at a glance' },
  '/devices': { title: 'Devices', subtitle: 'Paired and discovered devices on your local network' },
  '/pairing': { title: 'Pairing', subtitle: 'Generate a code to pair, or accept a pairing request' },
  '/clipboard': { title: 'Clipboard', subtitle: 'Shared clipboard history across all paired devices' },
  '/input': { title: 'Input', subtitle: 'Keyboard and mouse forwarding between paired devices' },
  '/files': { title: 'Files', subtitle: 'Shared file transfer across paired devices' },
  '/settings': { title: 'Settings', subtitle: 'Appearance, accessibility, and daemon configuration' },
  '/logs': { title: 'Logs', subtitle: 'Live daemon log output — refreshed every 3 seconds' },
  '/about': { title: 'About', subtitle: 'UniKey desktop shell and daemon information' },
};

export const TopBar: React.FC = () => {
  const { pathname } = useLocation();
  const { theme, setTheme, reduceMotion } = useAppearance();
  const { daemonReachable } = useDaemon();
  const inputRef = useRef<HTMLInputElement>(null);

  // ⌘K focuses search.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        inputRef.current?.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const meta = ROUTE_META[pathname] ?? ROUTE_META['/'];

  return (
    <motion.header
      initial={{ opacity: 0, y: -8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={reduceMotion ? { duration: 0.1 } : { duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
      className="flex flex-wrap items-center gap-x-5 gap-y-3 border-b border-black/[0.06] bg-white/25 px-6 py-4 backdrop-blur-2xl backdrop-saturate-150 dark:border-white/[0.08] dark:bg-white/[0.02]"
    >
      {/* Breadcrumb + title */}
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-1 text-[11.5px] font-medium tracking-[-0.01em] text-black/45 dark:text-white/45">
          <span>UniKey</span>
          <ChevronRight className="h-3 w-3 opacity-60" />
          <span className="text-indigo-500 dark:text-indigo-300">{meta.title}</span>
        </div>
        <h1 className="mt-0.5 truncate text-[17px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
          {meta.title}
        </h1>
        <p className="truncate text-[12px] leading-tight tracking-[-0.01em] text-black/45 dark:text-white/45">
          {meta.subtitle}
        </p>
      </div>

      {/* Search */}
      <label className="group relative flex min-w-0 items-center">
        <Search className="pointer-events-none absolute left-3 h-4 w-4 text-black/35 dark:text-white/35" />
        <input
          ref={inputRef}
          type="search"
          placeholder="Search devices, clipboard…"
          aria-label="Search"
          className="h-9 w-52 rounded-full border border-white/60 bg-black/[0.04] pl-9 pr-12 text-[13px] tracking-[-0.01em] text-neutral-900 outline-none transition-all duration-200 placeholder:text-black/35 focus:w-64 focus:border-indigo-400/40 focus:bg-black/[0.06] focus:ring-2 focus:ring-indigo-400/30 dark:border-white/10 dark:bg-white/[0.06] dark:text-white dark:placeholder:text-white/35"
        />
        <kbd className="pointer-events-none absolute right-2.5 rounded-[6px] border border-black/10 bg-white/70 px-1.5 py-0.5 text-[10px] font-semibold tracking-[-0.01em] text-black/45 shadow-[0_1px_1px_rgba(31,34,55,0.08)] dark:border-white/15 dark:bg-white/10 dark:text-white/45">
          ⌘K
        </kbd>
      </label>

      {/* Daemon status chip */}
      <div
        className={[
          'flex items-center gap-2 rounded-full border border-white/60 bg-white/50 py-1.5 pl-2.5 pr-3 shadow-[0_1px_2px_rgba(31,34,55,0.06)] backdrop-blur-xl',
          'dark:border-white/10 dark:bg-white/[0.06]',
        ].join(' ')}
      >
        <span className="relative flex h-2 w-2">
          {daemonReachable && (
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-500 opacity-60" />
          )}
          <span
            className={[
              'relative inline-flex h-2 w-2 rounded-full',
              daemonReachable ? 'bg-emerald-500' : 'bg-rose-500',
            ].join(' ')}
          />
        </span>
        <span className="text-[12px] font-medium leading-none tracking-[-0.01em] text-neutral-900 dark:text-white">
          {daemonReachable ? 'Daemon online' : 'Daemon offline'}
        </span>
        <span className="text-[12px] leading-none text-black/40 dark:text-white/40">·</span>
        <Lock className="h-3 w-3 text-black/40 dark:text-white/40" />
        <span className="text-[12px] font-medium leading-none tracking-[-0.01em] text-black/50 dark:text-white/50">
          E2E
        </span>
      </div>

      {/* Theme picker */}
      <Segmented<Theme>
        ariaLabel="Theme"
        layoutId="seg-theme-topbar"
        value={theme}
        onChange={setTheme}
        reduceMotion={reduceMotion}
        options={[
          { value: 'system', label: 'System' },
          { value: 'light', label: 'Light' },
          { value: 'dark', label: 'Dark' },
        ]}
      />

      {/* Avatar */}
      <span
        title="Host device"
        className="grid h-9 w-9 place-items-center rounded-full bg-gradient-to-br from-indigo-500/20 to-violet-500/20 text-[12px] font-bold tracking-tight text-indigo-600 ring-1 ring-inset ring-indigo-500/20 dark:text-indigo-300"
      >
        UK
      </span>
    </motion.header>
  );
};
