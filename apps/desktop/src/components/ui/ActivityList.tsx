/**
 * ActivityList.tsx
 * ----------------
 * A "Recent activity" glass card: timestamped rows with functional tint
 * tiles. Rows lift on hover and press on pointer-down. Ported from the
 * app-ui study.
 */

import React from 'react';
import { motion } from 'motion/react';
import { GlassCard, IconTile } from './primitives';

export interface ActivityItem {
  id: string;
  icon: React.ReactNode;
  tint: string;
  title: string;
  detail: string;
  time: string;
  live?: boolean;
}

export function ActivityList({
  title,
  items,
  footer,
  ariaLabel,
}: {
  title: string;
  items: ActivityItem[];
  footer?: string;
  ariaLabel?: string;
}) {
  return (
    <GlassCard ariaLabel={ariaLabel} className="flex flex-col">
      <h3 className="px-4 pt-4 text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
        {title}
      </h3>
      <div className="flex flex-1 flex-col gap-0.5 p-2">
        {items.map((item) => (
          <motion.div
            key={item.id}
            whileTap={{ scale: 0.995 }}
            className="flex items-center gap-3 rounded-xl px-2.5 py-2 transition-colors duration-150 hover:bg-black/[0.03] dark:hover:bg-white/[0.04]"
          >
            <IconTile icon={item.icon} tint={item.tint} size="sm" />
            <div className="min-w-0 flex-1">
              <p className="truncate text-[13px] font-medium leading-tight tracking-[-0.01em] text-neutral-900 dark:text-white">
                {item.title}
                {item.live && (
                  <span className="ml-1.5 inline-block h-1.5 w-1.5 animate-pulse rounded-full bg-emerald-500 align-middle" />
                )}
              </p>
              <p className="mt-0.5 truncate text-[11.5px] leading-tight tracking-[-0.01em] text-black/45 dark:text-white/45">
                {item.detail}
              </p>
            </div>
            <span className="shrink-0 text-[11.5px] font-medium tabular-nums tracking-[-0.01em] text-black/40 dark:text-white/40">
              {item.time}
            </span>
          </motion.div>
        ))}
      </div>
      {footer && (
        <p className="border-t border-black/[0.06] px-4 py-3 text-[12px] leading-tight tracking-[-0.01em] text-black/40 dark:border-white/[0.08] dark:text-white/40">
          {footer}
        </p>
      )}
    </GlassCard>
  );
}
