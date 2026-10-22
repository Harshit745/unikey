/**
 * Button.tsx
 * ----------
 * Action buttons in the app-ui glass language. Primary = indigo-filled,
 * secondary = frosted glass, ghost = text-only. Focus rings are visible.
 */

import React from 'react';

type Variant = 'primary' | 'secondary' | 'ghost';
type Size = 'sm' | 'md';

const VARIANTS: Record<Variant, string> = {
  primary:
    'bg-indigo-500 text-white shadow-[0_1px_2px_rgba(79,70,229,0.4),0_6px_16px_-6px_rgba(99,102,241,0.5)] hover:bg-indigo-400 active:bg-indigo-600',
  secondary:
    'bg-black/[0.05] text-neutral-900 hover:bg-black/[0.09] dark:bg-white/[0.08] dark:text-white dark:hover:bg-white/[0.14]',
  ghost: 'text-black/55 hover:bg-black/[0.05] hover:text-black/80 dark:text-white/55 dark:hover:bg-white/[0.07] dark:hover:text-white/85',
};

const SIZES: Record<Size, string> = {
  sm: 'h-8 px-3.5 text-[12.5px] rounded-lg',
  md: 'h-9 px-4 text-[13px] rounded-xl',
};

export const Button: React.FC<
  React.ButtonHTMLAttributes<HTMLButtonElement> & { variant?: Variant; size?: Size }
> = ({ variant = 'primary', size = 'md', className = '', type = 'button', ...rest }) => (
  <button
    type={type}
    className={[
      'inline-flex shrink-0 items-center justify-center gap-1.5 font-medium tracking-[-0.01em] outline-none transition-colors duration-150',
      'focus-visible:ring-2 focus-visible:ring-indigo-400/70',
      'disabled:cursor-not-allowed disabled:opacity-40',
      VARIANTS[variant],
      SIZES[size],
      className,
    ].join(' ')}
    {...rest}
  />
);
