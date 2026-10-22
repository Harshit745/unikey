/**
 * controls.tsx
 * ------------
 * Form controls in the glass language: a labeled Field wrapper, a text
 * input, and a select. Inputs match the TopBar search field (soft field
 * bg, indigo focus ring).
 */

import React from 'react';

const fieldCls = [
  'h-9 w-full rounded-xl border border-white/60 bg-black/[0.04] px-3 text-[13px] tracking-[-0.01em]',
  'text-neutral-900 outline-none transition-all duration-150',
  'placeholder:text-black/35',
  'focus:border-indigo-400/40 focus:bg-black/[0.06] focus:ring-2 focus:ring-indigo-400/30',
  'dark:border-white/10 dark:bg-white/[0.06] dark:text-white dark:placeholder:text-white/35',
].join(' ');

export const Field: React.FC<{ label: string; hint?: string; children: React.ReactNode }> = ({
  label,
  hint,
  children,
}) => (
  <label className="block">
    <span className="mb-1.5 block text-[11px] font-semibold uppercase tracking-[0.1em] text-black/45 dark:text-white/45">
      {label}
    </span>
    {children}
    {hint && (
      <span className="mt-1 block text-[11.5px] leading-tight tracking-[-0.01em] text-black/40 dark:text-white/40">
        {hint}
      </span>
    )}
  </label>
);

export const TextInput: React.FC<React.InputHTMLAttributes<HTMLInputElement>> = (props) => (
  <input {...props} className={[fieldCls, props.className ?? ''].join(' ')} />
);

export const Select: React.FC<React.SelectHTMLAttributes<HTMLSelectElement>> = (props) => (
  <select {...props} className={[fieldCls, 'cursor-pointer', props.className ?? ''].join(' ')} />
);

/** Centered empty-state block shown inside cards when there's no data yet. */
export const EmptyState: React.FC<{
  icon: React.ReactNode;
  title: string;
  description?: string;
  className?: string;
}> = ({ icon, title, description, className = '' }) => (
  <div className={['px-6 py-10 text-center', className].join(' ')}>
    <div className="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-[13px] bg-black/[0.04] text-black/40 dark:bg-white/[0.06] dark:text-white/40">
      {icon}
    </div>
    <p className="text-[14px] font-semibold leading-tight tracking-[-0.01em] text-neutral-900 dark:text-white">
      {title}
    </p>
    {description && (
      <p className="mx-auto mt-1 max-w-sm text-[12.5px] leading-snug tracking-[-0.01em] text-black/45 dark:text-white/45">
        {description}
      </p>
    )}
  </div>
);
