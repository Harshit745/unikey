/**
 * TopBar.tsx
 * ----------
 * Compact window header with the current page, route search, and the live
 * daemon state. Theme controls remain in Settings to keep this toolbar quiet.
 */

import React, { useEffect, useMemo, useRef, useState } from 'react';
import { useLocation, useNavigate } from 'react-router-dom';
import { motion } from 'motion/react';
import { Search } from 'lucide-react';
import { useAppearance } from '@/context/AppearanceContext';
import { useDaemon } from '@/context/DaemonContext';

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
  const navigate = useNavigate();
  const { reduceMotion } = useAppearance();
  const { daemonReachable } = useDaemon();
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState('');
  const [searchOpen, setSearchOpen] = useState(false);

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
  const results = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return [];
    return Object.entries(ROUTE_META).filter(([, item]) =>
      `${item.title} ${item.subtitle}`.toLowerCase().includes(needle),
    );
  }, [query]);

  const goTo = (path: string) => {
    navigate(path);
    setQuery('');
    setSearchOpen(false);
  };

  return (
    <motion.header
      initial={{ opacity: 0, y: -8 }}
      animate={{ opacity: 1, y: 0 }}
      transition={reduceMotion ? { duration: 0.1 } : { duration: 0.4, ease: [0.22, 1, 0.36, 1] }}
      className="app-topbar z-20 flex min-h-[76px] flex-wrap items-center gap-x-5 gap-y-3 px-7 py-3"
    >
      <div className="topbar-title min-w-0 flex-1">
        <h1 className="truncate text-[18px] font-semibold leading-tight tracking-[-0.025em] text-[var(--ink)]">
          {meta.title}
        </h1>
        <p className="mt-1 truncate text-[12px] leading-tight tracking-[-0.01em] text-[var(--ink-2)]">
          {meta.subtitle}
        </p>
      </div>

      {/* Search */}
      <div className="topbar-search relative min-w-0">
        <label className="relative flex items-center">
        <Search className="pointer-events-none absolute left-3 h-3.5 w-3.5 text-[var(--ink-3)]" />
        <input
          ref={inputRef}
          type="search"
          value={query}
          onChange={event => { setQuery(event.target.value); setSearchOpen(true); }}
          onFocus={() => setSearchOpen(true)}
          onKeyDown={event => {
            if (event.key === 'Enter' && results[0]) goTo(results[0][0]);
            if (event.key === 'Escape') { setSearchOpen(false); inputRef.current?.blur(); }
          }}
          placeholder="Search screens"
          aria-label="Search screens"
          aria-expanded={searchOpen && query.length > 0}
          className="h-9 w-52 rounded-[9px] border border-[var(--hairline)] bg-[var(--field-bg)] pl-9 pr-11 text-[12.5px] text-[var(--ink)] outline-none transition-[width,border-color,box-shadow] duration-150 placeholder:text-[var(--ink-3)] focus:w-60 focus:border-[var(--accent)]"
        />
        <kbd className="pointer-events-none absolute right-2.5 rounded-[5px] border border-[var(--hairline)] bg-[var(--surface-secondary)] px-1.5 py-0.5 text-[10px] font-medium text-[var(--ink-3)]">
          ⌘K
        </kbd>
        </label>
        {searchOpen && query && (
          <div className="surface-card absolute right-0 top-11 z-50 w-72 overflow-hidden p-1.5" role="listbox">
            {results.length ? results.slice(0, 6).map(([path, item]) => (
              <button key={path} type="button" onMouseDown={() => goTo(path)} className="block w-full rounded-[8px] px-3 py-2 text-left hover:bg-[var(--row-hover)]">
                <span className="block text-[13px] font-medium text-[var(--ink)]">{item.title}</span>
                <span className="mt-0.5 block truncate text-[11px] text-[var(--ink-2)]">{item.subtitle}</span>
              </button>
            )) : <p className="px-3 py-3 text-[12px] text-[var(--ink-2)]">No matching screen</p>}
          </div>
        )}
      </div>

      {/* Daemon status chip */}
      <div
        className={[
          'flex items-center gap-2 rounded-full border border-[var(--hairline)] bg-[var(--surface)] py-1.5 pl-2.5 pr-3',
        ].join(' ')}
      >
        <span className="relative flex h-2 w-2">
          {daemonReachable && (
            <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-[var(--ok)] opacity-50" />
          )}
          <span
            className={[
              'relative inline-flex h-2 w-2 rounded-full',
              daemonReachable ? 'bg-[var(--ok)]' : 'bg-[var(--danger)]',
            ].join(' ')}
          />
        </span>
        <span className="text-[12px] font-medium leading-none tracking-[-0.01em] text-[var(--ink)]">
          {daemonReachable ? 'Daemon online' : 'Daemon offline'}
        </span>
      </div>
    </motion.header>
  );
};
