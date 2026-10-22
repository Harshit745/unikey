/**
 * Sidebar.tsx
 * -----------
 * macOS-style glass navigation pane, ported from the app-ui study. The
 * active route gets a frosted pill that glides between targets via a
 * shared layoutId; the rail collapses to icons on a spring. Nav keeps the
 * real app's 9 routes across three groups.
 */

import React from 'react';
import { NavLink } from 'react-router-dom';
import { motion } from 'motion/react';
import {
  LayoutDashboard,
  MonitorSmartphone,
  KeyRound,
  ClipboardList,
  Keyboard,
  FolderUp,
  Settings,
  ScrollText,
  Info,
  ChevronsLeft,
  ChevronsRight,
  KeyRound as Logo,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import { useAppearance } from '@/context/AppearanceContext';
import { useDaemon } from '@/context/DaemonContext';
import { usePoll } from '@/hooks/usePoll';
import { api } from '@/api/client';
import type { DeviceInfo } from '@/api/types';

type NavItem = {
  to: string;
  label: string;
  icon: LucideIcon;
  end?: boolean;
};

const NAV_GROUPS: { label: string; items: NavItem[] }[] = [
  {
    label: 'Overview',
    items: [
      { to: '/', label: 'Dashboard', icon: LayoutDashboard, end: true },
      { to: '/devices', label: 'Devices', icon: MonitorSmartphone },
      { to: '/pairing', label: 'Pairing', icon: KeyRound },
    ],
  },
  {
    label: 'Sharing',
    items: [
      { to: '/clipboard', label: 'Clipboard', icon: ClipboardList },
      { to: '/input', label: 'Input', icon: Keyboard },
      { to: '/files', label: 'Files', icon: FolderUp },
    ],
  },
  {
    label: 'System',
    items: [
      { to: '/settings', label: 'Settings', icon: Settings },
      { to: '/logs', label: 'Logs', icon: ScrollText },
      { to: '/about', label: 'About', icon: Info },
    ],
  },
];

export const Sidebar: React.FC = () => {
  const { collapsed, setCollapsed } = useAppearance();
  const { daemonReachable } = useDaemon();
  const { data: localDevice } = usePoll<DeviceInfo>(api.device, 5000);

  return (
    <motion.aside
      aria-label="Primary"
      animate={{ width: collapsed ? 68 : 232 }}
      transition={{ type: 'spring', bounce: 0, duration: 0.4 }}
      className="app-sidebar relative flex shrink-0 flex-col overflow-hidden"
    >
      {/* Brand */}
      <div className="flex items-center gap-3 px-3.5 pb-5 pt-5">
        <span className="grid h-9 w-9 shrink-0 place-items-center rounded-[10px] bg-[var(--brand-bg)] text-[var(--brand-fg)]">
          <Logo className="h-[18px] w-[18px]" strokeWidth={2} />
        </span>
        <div className="min-w-0 overflow-hidden">
          <p className="truncate text-[15px] font-semibold leading-tight tracking-[-0.02em] text-[var(--ink)]">
            UniKey
          </p>
          <p className="truncate text-[11.5px] leading-tight tracking-[-0.01em] text-[var(--ink-2)]">
            Device workspace
          </p>
        </div>
      </div>

      {/* Nav */}
      <nav className="flex-1 space-y-5 overflow-y-auto overflow-x-hidden px-2.5">
        {NAV_GROUPS.map((group) => (
          <div key={group.label}>
            <p
              className={[
                'mb-1.5 px-2 text-[10px] font-semibold uppercase tracking-[0.12em] text-[var(--ink-3)]',
                collapsed ? 'hidden' : 'block',
              ].join(' ')}
            >
              {group.label}
            </p>
            <ul className="space-y-0.5">
              {group.items.map((item) => {
                const Icon = item.icon;
                return (
                  <li key={item.to} className="relative">
                    <NavLink
                      to={item.to}
                      end={item.end}
                      title={collapsed ? item.label : undefined}
                      className={({ isActive }) =>
                        [
                          'group relative flex w-full items-center gap-2.5 rounded-[9px] px-2 py-2 text-left outline-none',
                          'transition-colors duration-150',
                          !isActive && 'hover:bg-[var(--nav-hover)]',
                        ].join(' ')
                      }
                    >
                      {({ isActive }) => (
                        <>
                          {isActive && (
                            <motion.span
                              layoutId="nav-pill"
                              transition={{ type: 'spring', bounce: 0, duration: 0.5 }}
                              className="absolute inset-0 rounded-[9px] bg-[var(--nav-pill)]"
                            />
                          )}
                          <span
                            className={[
                              'relative grid h-6 w-6 shrink-0 place-items-center transition-colors duration-150',
                              isActive
                                ? 'text-[var(--accent)]'
                                : 'text-[var(--ink-2)] group-hover:text-[var(--ink)]',
                            ].join(' ')}
                          >
                            <Icon className="h-4 w-4" strokeWidth={2} />
                          </span>
                          <span className="relative min-w-0 flex-1 overflow-hidden">
                            <span
                              className={[
                                'truncate text-[13px] font-medium leading-none tracking-[-0.01em]',
                                isActive ? 'text-[var(--ink)]' : 'text-[var(--ink-2)]',
                                collapsed ? 'hidden' : 'block',
                              ].join(' ')}
                            >
                              {item.label}
                            </span>
                          </span>
                        </>
                      )}
                    </NavLink>
                  </li>
                );
              })}
            </ul>
          </div>
        ))}
      </nav>

      {/* Foot — host + collapse */}
      <div className="border-t border-[var(--hairline)] p-2.5">
        <div className={collapsed ? 'flex items-center justify-center gap-3' : 'flex items-center gap-3'}>
          {!collapsed && (
            <span className="grid h-8 w-8 shrink-0 place-items-center rounded-full bg-[var(--surface-tertiary)] text-[11px] font-semibold tracking-tight text-[var(--ink-2)] ring-1 ring-inset ring-[var(--hairline)]">
              {(localDevice?.name || 'UK').slice(0, 2).toUpperCase()}
            </span>
          )}
          {!collapsed && (
            <div className="min-w-0 flex-1 overflow-hidden">
              <p className="truncate text-[12.5px] font-medium leading-tight tracking-[-0.01em] text-[var(--ink)]">
                {localDevice?.name || 'This device'}
              </p>
              <p className="flex items-center gap-1.5 truncate text-[11px] leading-tight text-[var(--ink-2)]">
                <span
                  className={[
                    'h-1.5 w-1.5 shrink-0 rounded-full',
                    daemonReachable ? 'bg-[var(--ok)]' : 'bg-[var(--danger)]',
                  ].join(' ')}
                />
                {daemonReachable ? (localDevice?.platform || 'Daemon online') : 'Daemon offline'}
              </p>
            </div>
          )}
          <button
            type="button"
            onClick={() => setCollapsed((c) => !c)}
            aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            className="grid h-8 w-8 shrink-0 place-items-center rounded-[8px] text-[var(--ink-3)] transition-colors duration-150 hover:bg-[var(--nav-hover)] hover:text-[var(--ink)] active:scale-95"
          >
            {collapsed ? <ChevronsRight className="h-4 w-4" /> : <ChevronsLeft className="h-4 w-4" />}
          </button>
        </div>
      </div>
    </motion.aside>
  );
};
