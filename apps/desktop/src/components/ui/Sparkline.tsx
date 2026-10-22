/**
 * Sparkline.tsx
 * -------------
 * A tiny smoothed area chart for stat cards. Draws a Catmull-Rom → Bézier
 * path so the line is continuous and soft. Ported verbatim from the app-ui
 * study.
 */

import { useId } from 'react';

function smoothPath(pts: { x: number; y: number }[]): string {
  if (pts.length < 2) return '';
  let d = `M ${pts[0].x},${pts[0].y}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[Math.max(i - 1, 0)];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[Math.min(i + 2, pts.length - 1)];
    const c1x = p1.x + (p2.x - p0.x) / 6;
    const c1y = p1.y + (p2.y - p0.y) / 6;
    const c2x = p2.x - (p3.x - p1.x) / 6;
    const c2y = p2.y - (p3.y - p1.y) / 6;
    d += ` C ${c1x},${c1y} ${c2x},${c2y} ${p2.x},${p2.y}`;
  }
  return d;
}

export function Sparkline({
  data,
  color = 'indigo',
  width = 112,
  height = 34,
  className = '',
}: {
  data: number[];
  color?: 'indigo' | 'emerald' | 'sky' | 'amber' | 'violet';
  width?: number;
  height?: number;
  className?: string;
}) {
  const id = useId().replace(/[^a-zA-Z0-9]/g, '');
  const min = Math.min(...data);
  const max = Math.max(...data);
  const span = max - min || 1;
  const pad = 3;

  const pts = data.map((v, i) => ({
    x: pad + (i * (width - pad * 2)) / (data.length - 1),
    y: pad + (1 - (v - min) / span) * (height - pad * 2),
  }));
  const line = smoothPath(pts);
  const area = `${line} L ${pts[pts.length - 1].x},${height} L ${pts[0].x},${height} Z`;

  const hex =
    color === 'emerald' ? '#10b981' : color === 'sky' ? '#0ea5e9' : color === 'amber' ? '#f59e0b' : color === 'violet' ? '#8b5cf6' : '#6366f1';

  return (
    <svg
      width={width}
      height={height}
      viewBox={`0 0 ${width} ${height}`}
      className={className}
      aria-hidden="true"
    >
      <defs>
        <linearGradient id={`sp-${id}`} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor={hex} stopOpacity="0.22" />
          <stop offset="100%" stopColor={hex} stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={area} fill={`url(#sp-${id})`} />
      <path
        d={line}
        fill="none"
        stroke={hex}
        strokeWidth={1.75}
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}
