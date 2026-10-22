<!-- BEGIN:nextjs-agent-rules -->
This is NOT the Next.js you know. This version (Next.js 16.2) has breaking changes vs your training data. Read the relevant guide in `node_modules/next/dist/docs/` before writing code.

This project uses **Tailwind CSS v4** (CSS-first config via `@theme` in `globals.css`, NOT a `tailwind.config.ts`). If you need custom theme values, add them as `@theme inline { }` blocks in the CSS, not in a JS config.

<!-- END:nextjs-agent-rules -->

<!-- BEGIN:project-rules -->
# UniKey Website

**Tech stack:** Next.js 16 (App Router), React 19, TypeScript, Tailwind CSS v4, Framer Motion

**Design system:** All tokens live in `src/app/globals.css` under `@theme inline {}`. Uses custom color names like `canvas`, `violet-*`, `cyan-*`, `surface`, `text-*`. Utility classes: `.glass`, `.glass-card`, `.text-gradient`, `.text-gradient-white`, `.glow-violet`, `.glow-cyan`.

**Components:** Reusable UI in `src/components/ui/`. Page sections in `src/components/sections/`. Main page at `src/app/page.tsx`. Layout with Inter font + SEO in `src/app/layout.tsx`.

**Available sections:** Hero, Problem, Features, Pricing, FAQ, CTA, Footer. All use `"use client"` for Framer Motion animations. Scroll-triggered reveals via `useInView`.

**Commands:**
- `npm run dev` — dev server with Turbopack
- `npm run build` — production build
- `npm run lint` — ESLint
<!-- END:project-rules -->
