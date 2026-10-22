import React from 'react';

interface Props {
  icon: string;
  title: string;
  description?: string;
}

/**
 * Shown on pages when the daemon is offline and the page can't load its
 * data. Rendered inside a glass card.
 */
export const Placeholder: React.FC<Props> = ({ icon, title, description }) => (
  <div className="rounded-3xl border border-white/60 bg-white/55 px-6 py-12 text-center shadow-[inset_0_1px_0_rgba(255,255,255,0.7),0_24px_60px_-16px_rgba(31,34,55,0.28)] backdrop-blur-2xl dark:border-white/10 dark:bg-[#1b1b1e]/55 dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.12),0_24px_60px_-16px_rgba(0,0,0,0.6)]">
  <div className="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-[13px] bg-black/[0.04] text-[22px] leading-none dark:bg-white/[0.06]">
    {icon}
  </div>
  <p className="text-[15px] font-semibold leading-tight tracking-[-0.02em] text-neutral-900 dark:text-white">
    {title}
  </p>
  {description && (
    <p className="mx-auto mt-1 max-w-sm text-[13px] leading-snug tracking-[-0.01em] text-black/50 dark:text-white/50">
      {description}
    </p>
  )}
</div>
);
