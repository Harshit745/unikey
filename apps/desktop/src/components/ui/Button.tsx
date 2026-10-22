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
    'bg-[var(--accent)] text-white hover:bg-[var(--accent-strong)] active:brightness-90',
  secondary:
    'border border-[var(--hairline)] bg-[var(--surface)] text-[var(--ink)] hover:bg-[var(--row-hover)]',
  ghost: 'text-[var(--ink-2)] hover:bg-[var(--row-hover)] hover:text-[var(--ink)]',
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
      'focus-visible:ring-2 focus-visible:ring-[var(--accent)]',
      'disabled:cursor-not-allowed disabled:opacity-40',
      VARIANTS[variant],
      SIZES[size],
      className,
    ].join(' ')}
    {...rest}
  />
);
