/**
 * Badge.tsx
 * ---------
 * Small status pill. Tones follow the functional-color language: green
 * for success/online, amber for warnings, rose for errors, indigo for
 * active states, neutral for everything else.
 */

import React from 'react';

type Tone = 'success' | 'warning' | 'error' | 'accent' | 'neutral';

const TONES: Record<Tone, string> = {
  success:
    'bg-emerald-500/12 text-emerald-700 dark:bg-emerald-400/20 dark:text-emerald-300',
  warning:
    'bg-amber-500/12 text-amber-700 dark:bg-amber-400/20 dark:text-amber-300',
  error:
    'bg-rose-500/12 text-rose-700 dark:bg-rose-400/20 dark:text-rose-300',
  accent:
    'bg-indigo-500/12 text-indigo-700 dark:bg-indigo-400/20 dark:text-indigo-300',
  neutral:
    'bg-black/[0.05] text-black/55 dark:bg-white/[0.08] dark:text-white/55',
};

export const Badge: React.FC<{ tone?: Tone; className?: string; children: React.ReactNode }> = ({
  tone = 'neutral',
  className = '',
  children,
}) => (
  <span
    className={[
      'inline-flex items-center gap-1 rounded-full px-2.5 py-1 text-[11px] font-semibold leading-none tracking-[-0.01em]',
      TONES[tone],
      className,
    ].join(' ')}
  >
    {children}
  </span>
);
