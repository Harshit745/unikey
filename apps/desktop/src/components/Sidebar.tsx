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

  return (
    <motion.aside
      aria-label="Primary"
      animate={{ width: collapsed ? 76 : 252 }}
      transition={{ type: 'spring', bounce: 0, duration: 0.55 }}
      className="relative flex shrink-0 flex-col overflow-hidden border-r border-black/[0.06] bg-white/30 backdrop-blur-2xl backdrop-saturate-150 dark:border-white/[0.08] dark:bg-white/[0.02]"
    >
      {/* Brand */}
      <div className="flex items-center gap-3 px-4 pb-5 pt-5">
        <span className="grid h-10 w-10 shrink-0 place-items-center rounded-[12px] bg-neutral-900 text-white shadow-[inset_0_1px_0_rgba(255,255,255,0.2)] dark:bg-white/10">
          <Logo className="h-5 w-5" strokeWidth={2} />
        </span>
        <div className="min-w-0 overflow-hidden">
          <p className="truncate text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
            UniKey
          </p>
          <p className="truncate text-[11.5px] leading-tight tracking-[-0.01em] text-black/45 dark:text-white/45">
            Workspace mesh
          </p>
        </div>
      </div>

      {/* Nav */}
      <nav className="flex-1 space-y-5 overflow-hidden px-3">
        {NAV_GROUPS.map((group) => (
          <div key={group.label}>
            <p
              className={[
                'mb-1.5 px-2 text-[11px] font-semibold uppercase tracking-[0.14em] text-black/35 dark:text-white/35',
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
                          'group relative flex w-full items-center gap-3 rounded-[11px] px-2.5 py-[9px] text-left outline-none',
                          'transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-indigo-400/70',
                          !isActive && 'hover:bg-black/[0.04] dark:hover:bg-white/[0.05]',
                        ].join(' ')
                      }
                    >
                      {({ isActive }) => (
                        <>
                          {isActive && (
                            <motion.span
                              layoutId="nav-pill"
                              transition={{ type: 'spring', bounce: 0, duration: 0.5 }}
                              className="absolute inset-0 rounded-[11px] bg-white/80 shadow-[0_1px_2px_rgba(31,34,55,0.07),0_6px_18px_-8px_rgba(31,34,55,0.18)] dark:bg-white/[0.13] dark:shadow-[0_1px_2px_rgba(0,0,0,0.3),0_8px_20px_-10px_rgba(0,0,0,0.5)]"
                            />
                          )}
                          <span
                            className={[
                              'grid h-[26px] w-[26px] shrink-0 place-items-center transition-colors duration-150',
                              isActive
                                ? 'text-indigo-600 dark:text-indigo-300'
                                : 'text-black/50 group-hover:text-black/75 dark:text-white/50 dark:group-hover:text-white/80',
                            ].join(' ')}
                          >
                            <Icon className="h-[18px] w-[18px]" strokeWidth={2} />
                          </span>
                          <span className="relative min-w-0 flex-1 overflow-hidden">
                            <span
                              className={[
                                'truncate text-[13.5px] font-medium leading-none tracking-[-0.01em]',
                                isActive ? 'text-neutral-900 dark:text-white' : 'text-black/60 dark:text-white/60',
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
      <div className="border-t border-black/[0.06] p-3 dark:border-white/[0.08]">
        <div className={collapsed ? 'flex items-center justify-center gap-3' : 'flex items-center gap-3'}>
          {!collapsed && (
            <span className="grid h-9 w-9 shrink-0 place-items-center rounded-full bg-gradient-to-br from-indigo-500/20 to-violet-500/20 text-[12px] font-bold tracking-tight text-indigo-600 ring-1 ring-inset ring-indigo-500/20 dark:text-indigo-300">
              UK
            </span>
          )}
          {!collapsed && (
            <div className="min-w-0 flex-1 overflow-hidden">
              <p className="truncate text-[13px] font-medium leading-tight tracking-[-0.01em] text-neutral-900 dark:text-white">
                This device
              </p>
              <p className="flex items-center gap-1.5 truncate text-[11.5px] leading-tight text-black/45 dark:text-white/45">
                <span
                  className={[
                    'h-1.5 w-1.5 shrink-0 rounded-full',
                    daemonReachable ? 'bg-emerald-500 dark:bg-emerald-400' : 'bg-rose-500 dark:bg-rose-400',
                  ].join(' ')}
                />
                {daemonReachable ? 'Daemon online' : 'Daemon offline'}
              </p>
            </div>
          )}
          <button
            type="button"
            onClick={() => setCollapsed((c) => !c)}
            aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
            className="grid h-8 w-8 shrink-0 place-items-center rounded-full text-black/40 transition-colors duration-150 hover:bg-black/[0.06] active:scale-90 dark:text-white/40 dark:hover:bg-white/10"
          >
            {collapsed ? <ChevronsRight className="h-4 w-4" /> : <ChevronsLeft className="h-4 w-4" />}
          </button>
        </div>
      </div>
    </motion.aside>
  );
};
