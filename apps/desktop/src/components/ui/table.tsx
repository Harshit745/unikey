/**
 * table.tsx
 * ---------
 * Data-table primitives in the glass language. Hairline rows, muted
 * uppercase headers, quiet hover tint — no visible card chrome so tables
 * sit naturally inside a GlassCard.
 */

import React from 'react';

export const Table: React.FC<{ children: React.ReactNode }> = ({ children }) => (
  <div className="w-full overflow-x-auto">
    <table className="w-full border-collapse text-[13px]"><tbody>{children}</tbody></table>
  </div>
);

export const Th: React.FC<{ className?: string; children: React.ReactNode }> = ({
  className = '',
  children,
}) => (
  <th
    className={[
      'whitespace-nowrap border-b border-[var(--hairline)] px-4 py-2.5 text-left text-[10.5px] font-semibold uppercase tracking-[0.06em] text-[var(--ink-3)]',
      className,
    ].join(' ')}
  >
    {children}
  </th>
);

export const Td: React.FC<{ className?: string; children: React.ReactNode }> = ({
  className = '',
  children,
}) => (
  <td
    className={[
      'border-b border-[var(--hairline)] px-4 py-3 align-middle text-[var(--ink)]',
      className,
    ].join(' ')}
  >
    {children}
  </td>
);

export const Tr: React.FC<{ className?: string; children: React.ReactNode }> = ({
  className = '',
  children,
}) => (
  <tr
    className={[
      'transition-colors duration-100 last:border-0 hover:bg-[var(--row-hover)]',
      className,
    ].join(' ')}
  >
    {children}
  </tr>
);
