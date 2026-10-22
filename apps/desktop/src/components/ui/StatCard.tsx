/**
 * StatCard.tsx
 * ------------
 * A compact, solid dashboard summary with a functional icon, current value,
 * plain-language label, and optional supporting detail. Values render
 * immediately so live status is never temporarily misrepresented.
 */

import React from 'react';
import { IconTile } from './primitives';

export function StatCard({
  icon,
  tint,
  label,
  value,
  valueNode,
  format,
  delta,
  deltaTone = 'up',
  detail,
  tone = 'neutral',
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
  detail?: React.ReactNode;
  tone?: 'neutral' | 'success' | 'warning' | 'danger';
  reduceMotion: boolean;
  style?: React.CSSProperties;
}) {
  const shown = value ?? 0;
  const text = valueNode ?? (format ? format(shown) : Math.round(shown).toLocaleString());

  const deltaClass =
    deltaTone === 'up'
      ? 'text-emerald-600 dark:text-emerald-300'
      : deltaTone === 'down'
        ? 'text-rose-500 dark:text-rose-300'
        : 'text-black/50 dark:text-white/50';

  const toneClass = tone === 'success'
    ? 'text-[var(--ok)]'
    : tone === 'warning'
      ? 'text-[var(--warn)]'
      : tone === 'danger'
        ? 'text-[var(--danger)]'
        : 'text-[var(--ink)]';

  return (
    <article
      style={style}
      className="surface-card flex min-h-[138px] flex-1 flex-col justify-between gap-4 p-4"
    >
      <div className="flex items-start justify-between gap-2">
        <IconTile icon={icon} tint={tint} />
        {delta && (
          <span className={`rounded-full px-2 py-0.5 text-[11px] font-semibold leading-none tracking-[-0.01em] ${deltaClass} bg-black/[0.04] dark:bg-white/[0.06]`}>
            {delta}
          </span>
        )}
      </div>
      <div>
        <div>
          <p className={`text-[25px] font-semibold leading-none tracking-[-0.03em] tabular-nums ${toneClass}`}>
            {text}
          </p>
          <p className="mt-1.5 text-[12px] font-medium leading-tight tracking-[-0.01em] text-[var(--ink-2)]">
            {label}
          </p>
        </div>
        {detail && <p className="mt-1 text-[10.5px] leading-snug text-[var(--ink-3)]">{detail}</p>}
      </div>
    </article>
  );
}
