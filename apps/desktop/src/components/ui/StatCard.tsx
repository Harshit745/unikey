/**
 * StatCard.tsx
 * ------------
 * A translucent dashboard widget: functional icon tile, label, animated
 * count-up value (or a free-form valueNode for composite values like
 * "3/8" or "Active"), delta chip, and an optional sparkline. Reacts on
 * press, lifts on hover, and reads the global reduce-motion state.
 *
 * Ported from the app-ui study with `valueNode` and an optional spark.
 */

import React from 'react';
import { motion } from 'motion/react';
import { IconTile } from './primitives';
import { Sparkline } from './Sparkline';

/** §3 — ease-out count-up, or instant under reduced motion. */
function useCountUp(target: number, reduceMotion: boolean, duration = 900) {
  const [value, setValue] = React.useState(reduceMotion ? target : 0);
  const raf = React.useRef<number | null>(null);
  React.useEffect(() => {
    if (reduceMotion) {
      setValue(target);
      return;
    }
    const t0 = performance.now();
    const tick = (now: number) => {
      const p = Math.min((now - t0) / duration, 1);
      const eased = 1 - Math.pow(1 - p, 3);
      setValue(target * eased);
      if (p < 1) raf.current = requestAnimationFrame(tick);
    };
    raf.current = requestAnimationFrame(tick);
    return () => {
      if (raf.current) cancelAnimationFrame(raf.current);
    };
  }, [target, reduceMotion, duration]);
  return value;
}

export type SparkColor = 'indigo' | 'emerald' | 'sky' | 'amber' | 'violet';

export function StatCard({
  icon,
  tint,
  label,
  value,
  valueNode,
  format,
  delta,
  deltaTone = 'up',
  spark,
  sparkColor,
  reduceMotion,
  style,
}: {
  icon: React.ReactNode;
  tint: string;
  label: string;
  /** Numeric value — rendered with an ease-out count-up. */
  value?: number;
  /** Free-form display value — wins over the count-up when provided. */
  valueNode?: React.ReactNode;
  format?: (v: number) => string;
  delta?: string;
  deltaTone?: 'up' | 'down' | 'flat';
  spark?: number[];
  sparkColor?: SparkColor;
  reduceMotion: boolean;
  style?: React.CSSProperties;
}) {
  const shown = useCountUp(value ?? 0, reduceMotion);
  const text = valueNode ?? (format ? format(shown) : Math.round(shown).toLocaleString());

  const deltaClass =
    deltaTone === 'up'
      ? 'text-emerald-600 dark:text-emerald-300'
      : deltaTone === 'down'
        ? 'text-rose-500 dark:text-rose-300'
        : 'text-black/50 dark:text-white/50';

  return (
    <motion.div
      style={style}
      whileHover={{ y: -2 }}
      whileTap={{ scale: 0.985 }}
      transition={{ type: 'spring', bounce: 0, duration: 0.4 }}
      className="flex flex-1 flex-col justify-between gap-3 rounded-2xl border border-white/60 bg-white/55 p-4 shadow-[inset_0_1px_0_rgba(255,255,255,0.7),0_2px_6px_rgba(31,34,55,0.06),0_16px_40px_-18px_rgba(31,34,55,0.25)] backdrop-blur-xl backdrop-saturate-150 dark:border-white/10 dark:bg-white/[0.04] dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.1),0_16px_40px_-18px_rgba(0,0,0,0.55)]"
    >
      <div className="flex items-start justify-between gap-2">
        <IconTile icon={icon} tint={tint} />
        {delta && (
          <span className={`rounded-full px-2 py-0.5 text-[11px] font-semibold leading-none tracking-[-0.01em] ${deltaClass} bg-black/[0.04] dark:bg-white/[0.06]`}>
            {delta}
          </span>
        )}
      </div>
      <div className="flex items-end justify-between gap-3">
        <div className="min-w-0">
          <p className="text-[24px] font-semibold leading-none tracking-[-0.03em] tabular-nums text-neutral-900 dark:text-white">
            {text}
          </p>
          <p className="mt-1.5 truncate text-[12px] font-medium leading-tight tracking-[-0.01em] text-black/50 dark:text-white/50">
            {label}
          </p>
        </div>
        {spark && <Sparkline data={spark} color={sparkColor} className="shrink-0" />}
      </div>
    </motion.div>
  );
}
