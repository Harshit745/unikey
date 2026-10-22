/**
 * primitives.tsx
 * ---------------
 * Shared building blocks ported verbatim from the app-ui design study
 * (.claude/demo/app-ui/primitives.tsx), built on the same apple-design
 * rules as the Glass Settings study:
 *
 *   §12  Materials — translucent `backdrop-filter` layer, bright top edge,
 *        layered depth shadow.
 *   §2/5/6  Direct manipulation — switches drag 1:1, hand off release
 *        velocity into the spring, project momentum to decide the commit.
 *   §9  Rubber-banding — the knob resists past its bounds, never hard-stops.
 *   §14  Reduce Motion / Reduce Transparency honored globally.
 *   §15  Typography — size-specific tracking, tight leading.
 *
 * One deliberate fix vs. the demo: `Segmented` takes a `layoutId` prop
 * (default "segmented-indicator") so the TopBar theme picker and the
 * Settings page picker never share a motion layout id.
 */

import React from 'react';
import { motion, useMotionValue, useSpring } from 'motion/react';

/* ------------------------------------------------------------------ */
/*  Physics helpers                                                    */
/* ------------------------------------------------------------------ */

/** §9 — progressive resistance past a boundary instead of a hard stop. */
export function rubberband(overshoot: number, dimension: number, constant = 0.55) {
  return (overshoot * dimension * constant) / (dimension + constant * Math.abs(overshoot));
}

/** §6 — exponential-decay momentum projection (Apple's scroll form). */
export function project(initialVelocity: number, decelerationRate = 0.998) {
  return (initialVelocity / 1000) * (decelerationRate / (1 - decelerationRate));
}

/** §14 — live matchMedia hook. */
export function useMediaQuery(query: string) {
  const [matches, setMatches] = React.useState(
    () => typeof window !== 'undefined' && window.matchMedia(query).matches,
  );
  React.useEffect(() => {
    const mq = window.matchMedia(query);
    const onChange = () => setMatches(mq.matches);
    onChange();
    mq.addEventListener('change', onChange);
    return () => mq.removeEventListener('change', onChange);
  }, [query]);
  return matches;
}

/* ------------------------------------------------------------------ */
/*  SpringSwitch — draggable iOS-style switch (identical physics to    */
/*  the settings demo)                                                 */
/* ------------------------------------------------------------------ */

export const TRACK_W = 48;
const KNOB = 24;
const PAD = 2;
const TRAVEL = TRACK_W - PAD * 2 - KNOB; // 20px of knob travel

interface SpringSwitchProps {
  checked: boolean;
  onChange: (next: boolean) => void;
  label: string;
  reduceMotion: boolean;
  disabled?: boolean;
}

export function SpringSwitch({ checked, onChange, label, reduceMotion, disabled }: SpringSwitchProps) {
  const x = useMotionValue(checked ? TRAVEL : 0);
  const knobX = useSpring(
    x,
    reduceMotion ? { stiffness: 3000, damping: 90 } : { stiffness: 500, damping: 38 },
  );

  const drag = React.useRef<{
    startClientX: number;
    baseX: number;
    points: { t: number; x: number }[];
    moved: boolean;
    pointerId: number;
  } | null>(null);
  const pointerHandled = React.useRef(false);

  React.useEffect(() => {
    x.set(checked ? TRAVEL : 0);
  }, [checked, x]);

  const onPointerDown = (e: React.PointerEvent<HTMLButtonElement>) => {
    if (disabled || reduceMotion) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = {
      startClientX: e.clientX,
      baseX: x.get(),
      points: [{ t: e.timeStamp, x: x.get() }],
      moved: false,
      pointerId: e.pointerId,
    };
  };

  const onPointerMove = (e: React.PointerEvent<HTMLButtonElement>) => {
    const s = drag.current;
    if (!s || s.pointerId !== e.pointerId) return;
    let raw = s.baseX + (e.clientX - s.startClientX);
    if (raw < 0) raw = rubberband(raw, TRAVEL);
    else if (raw > TRAVEL) raw = TRAVEL + rubberband(raw - TRAVEL, TRAVEL);
    if (Math.abs(raw - s.baseX) > 3) s.moved = true;
    x.set(raw);
    s.points.push({ t: e.timeStamp, x: raw });
    if (s.points.length > 6) s.points.shift();
  };

  const onPointerUp = (e: React.PointerEvent<HTMLButtonElement>) => {
    const s = drag.current;
    if (!s || s.pointerId !== e.pointerId) return;
    drag.current = null;
    pointerHandled.current = true;
    const first = s.points[0];
    const last = s.points[s.points.length - 1];
    const velocity =
      s.points.length > 1 ? ((last.x - first.x) / Math.max(last.t - first.t, 1)) * 1000 : 0;
    const next = s.moved ? x.get() + project(velocity) > TRAVEL / 2 : !checked;
    onChange(next);
    x.set(next ? TRAVEL : 0);
  };

  const onPointerCancel = () => {
    drag.current = null;
    x.set(checked ? TRAVEL : 0);
  };

  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerCancel}
      onClick={() => {
        if (pointerHandled.current) {
          pointerHandled.current = false;
          return;
        }
        onChange(!checked);
        x.set(!checked ? TRAVEL : 0);
      }}
      className={[
        'relative h-[28px] w-[48px] shrink-0 rounded-full outline-none transition-colors duration-200',
        'focus-visible:ring-2 focus-visible:ring-[var(--accent)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--surface)]',
        disabled ? 'opacity-40' : 'cursor-pointer',
        checked
          ? 'bg-[var(--sw-on)]'
          : 'bg-[var(--sw-off)]',
      ].join(' ')}
      style={{ touchAction: 'none' }}
    >
      <motion.span
        style={{ x: knobX }}
        className="pointer-events-none absolute left-0 top-[2px] h-6 w-6 rounded-full bg-white shadow-[0_2px_5px_rgba(0,0,0,0.3)]"
      />
    </button>
  );
}

/* ------------------------------------------------------------------ */
/*  Segmented — animated system/light/dark picker                      */
/* ------------------------------------------------------------------ */

interface SegmentedProps<T extends string> {
  value: T;
  options: { value: T; label: string }[];
  onChange: (value: T) => void;
  reduceMotion: boolean;
  ariaLabel: string;
  /** Unique layout id so multiple pickers never share a gliding pill. */
  layoutId?: string;
  /** Extra classes for the root element. */
  className?: string;
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  reduceMotion,
  ariaLabel,
  layoutId = 'segmented-indicator',
  className = '',
}: SegmentedProps<T>) {
  return (
    <div
      role="tablist"
      aria-label={ariaLabel}
      className={[
        'flex shrink-0 rounded-[10px] bg-[var(--seg-bg)] p-[3px]',
        className,
      ].join(' ')}
    >
      {options.map((opt) => {
        const active = opt.value === value;
        return (
          <button
            key={opt.value}
            type="button"
            role="tab"
            aria-selected={active}
            onClick={() => onChange(opt.value)}
            className={[
              'relative rounded-[9px] px-3 py-[5px] text-[12.5px] font-medium outline-none',
              'transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-[var(--accent)]',
              active
                ? 'text-[var(--ink)]'
                : 'text-[var(--ink-2)] hover:text-[var(--ink)]',
            ].join(' ')}
          >
            {active && (
              <motion.span
                layoutId={layoutId}
                transition={
                  reduceMotion
                    ? { duration: 0.08 }
                    : { type: 'spring', bounce: 0, duration: 0.45 }
                }
                className="absolute inset-0 rounded-[8px] bg-[var(--seg-pill)] shadow-[var(--seg-pill-shadow)]"
              />
            )}
            <span className="relative z-10 whitespace-nowrap">{opt.label}</span>
          </button>
        );
      })}
    </div>
  );
}

/* ------------------------------------------------------------------ */
/*  IconTile — functional tinted squircle used across every row/card   */
/* ------------------------------------------------------------------ */

export function IconTile({
  icon,
  tint,
  size = 'md',
}: {
  icon: React.ReactNode;
  tint: string;
  size?: 'sm' | 'md' | 'lg';
}) {
  const box =
    size === 'sm' ? 'h-8 w-8 rounded-[9px]' : size === 'lg' ? 'h-11 w-11 rounded-[13px]' : 'h-9 w-9 rounded-[10px]';
  const svg = size === 'sm' ? 'h-4 w-4' : size === 'lg' ? 'h-[22px] w-[22px]' : 'h-[18px] w-[18px]';
  return (
    <span className={['grid shrink-0 place-items-center', box, tint].join(' ')}>
      {React.cloneElement(icon as React.ReactElement, { className: svg, strokeWidth: 2 })}
    </span>
  );
}

/* ------------------------------------------------------------------ */
/*  GlassCard — §12 material wrapper with optional header              */
/* ------------------------------------------------------------------ */

export function GlassCard({
  children,
  className = '',
  as: Tag = 'section',
  ariaLabel,
}: {
  children: React.ReactNode;
  className?: string;
  as?: 'section' | 'div' | 'article';
  ariaLabel?: string;
}) {
  return (
    <Tag
      aria-label={ariaLabel}
      className={[
        'surface-card relative overflow-hidden',
        className,
      ].join(' ')}
    >
      {children}
    </Tag>
  );
}

/* ------------------------------------------------------------------ */
/*  SectionHeader — eyebrow + title + subtitle                         */
/* ------------------------------------------------------------------ */

export function SectionHeader({
  eyebrow,
  title,
  description,
  right,
}: {
  eyebrow?: string;
  title: string;
  description?: string;
  right?: React.ReactNode;
}) {
  return (
    <div className="mb-4 flex flex-wrap items-end justify-between gap-4">
      <div>
        {eyebrow && (
          <p className="mb-1 text-[10px] font-semibold uppercase tracking-[0.13em] text-[var(--accent)]">
            {eyebrow}
          </p>
        )}
        <h2 className="text-[20px] font-semibold leading-tight tracking-[-0.025em] text-[var(--ink)]">
          {title}
        </h2>
        {description && (
          <p className="mt-1 text-[13.5px] leading-snug tracking-[-0.01em] text-[var(--ink-2)]">
            {description}
          </p>
        )}
      </div>
      {right}
    </div>
  );
}
