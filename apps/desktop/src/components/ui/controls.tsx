/**
 * controls.tsx
 * ------------
 * Form controls in the glass language: a labeled Field wrapper, a text
 * input, and a select. Inputs match the TopBar search field (soft field
 * bg, indigo focus ring).
 */

import React from 'react';

const fieldCls = [
  'h-9 w-full rounded-[9px] border border-[var(--hairline)] bg-[var(--field-bg)] px-3 text-[13px] tracking-[-0.01em]',
  'text-[var(--ink)] outline-none transition-all duration-150',
  'placeholder:text-[var(--ink-3)]',
  'focus:border-[var(--accent)] focus:ring-2 focus:ring-[var(--field-focus)]',
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
  action?: React.ReactNode;
  className?: string;
}> = ({ icon, title, description, action, className = '' }) => (
  <div className={['px-6 py-8 text-center', className].join(' ')}>
    <div className="mx-auto mb-3 grid h-10 w-10 place-items-center rounded-[11px] bg-[var(--surface-tertiary)] text-[var(--ink-3)]">
      {icon}
    </div>
    <p className="text-[14px] font-semibold leading-tight tracking-[-0.01em] text-[var(--ink)]">
      {title}
    </p>
    {description && (
      <p className="mx-auto mt-1 max-w-sm text-[12.5px] leading-snug tracking-[-0.01em] text-[var(--ink-2)]">
        {description}
      </p>
    )}
    {action && <div className="mt-4 flex justify-center">{action}</div>}
  </div>
);
