/**
 * DeviceCard.tsx
 * --------------
 * A connected/trusted machine in the mesh: platform tile, name, OS,
 * connection status with latency, role chip, and a forget action.
 * Ported from the app-ui study; the ellipsis button becomes a real
 * `onForget` action wired to the daemon's remove-device endpoint.
 */

import React from 'react';
import { motion } from 'motion/react';
import { Trash2 } from 'lucide-react';
import { GlassCard } from './primitives';

export type DeviceStatus = 'connected' | 'idle' | 'away';

const STATUS_META: Record<DeviceStatus, { label: string; dot: string; text: string }> = {
  connected: { label: 'Connected', dot: 'bg-emerald-500', text: 'text-emerald-600 dark:text-emerald-300' },
  idle: { label: 'Idle', dot: 'bg-amber-400', text: 'text-amber-600 dark:text-amber-300' },
  away: { label: 'Away', dot: 'bg-black/25 dark:bg-white/25', text: 'text-black/45 dark:text-white/45' },
};

export function DeviceCard({
  icon,
  tint,
  name,
  os,
  status,
  latency,
  role,
  onForget,
}: {
  icon: React.ReactNode;
  tint: string;
  name: string;
  os: string;
  status: DeviceStatus;
  latency: string;
  role: 'Host' | 'Peer';
  onForget?: () => void;
}) {
  const meta = STATUS_META[status];
  return (
    <motion.article
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.99 }}
      transition={{ type: 'spring', bounce: 0, duration: 0.4 }}
      className="surface-card group flex items-center gap-4 p-4"
    >
      <span className={`grid h-11 w-11 shrink-0 place-items-center rounded-[13px] ${tint}`}>
        {icon}
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-center gap-2">
          <h3 className="truncate text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
            {name}
          </h3>
          {role === 'Host' && (
            <span className="rounded-full bg-indigo-500/12 px-2 py-0.5 text-[10.5px] font-semibold uppercase leading-none tracking-[0.06em] text-indigo-600 dark:bg-indigo-400/20 dark:text-indigo-300">
              Host
            </span>
          )}
        </div>
        <p className="mt-1 flex items-center gap-2 truncate text-[12px] leading-tight tracking-[-0.01em] text-black/50 dark:text-white/50">
          {os}
          <span className="h-1 w-1 rounded-full bg-black/15 dark:bg-white/15" />
          <span className="tabular-nums">{latency} latency</span>
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-3">
        <div className="flex items-center gap-1.5">
          <span className={`h-2 w-2 rounded-full ${meta.dot}`} />
          <span className={`text-[12px] font-medium leading-none tracking-[-0.01em] ${meta.text}`}>
            {meta.label}
          </span>
        </div>
        {onForget && (
          <button
            type="button"
            onClick={onForget}
            aria-label={`Forget ${name}`}
            className="grid h-8 w-8 place-items-center rounded-full text-black/35 transition-colors duration-150 hover:bg-red-500/10 hover:text-red-600 active:scale-90 dark:text-white/35 dark:hover:bg-red-400/15 dark:hover:text-red-400"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        )}
      </div>
    </motion.article>
  );
}

export { GlassCard };
