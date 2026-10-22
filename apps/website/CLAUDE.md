# UniKey Website

## Tech Stack
- **Framework:** Next.js 16.2 (App Router) with Turbopack
- **Language:** TypeScript (strict)
- **Styling:** Tailwind CSS v4 (CSS-first config — see `@theme` in `globals.css`)
- **Animation:** Framer Motion (`motion.div`, `useInView`, `AnimatePresence`)
- **Icons:** lucide-react
- **UI Primitives:** @radix-ui/react-slot, class-variance-authority

## Design System
All tokens: `@theme inline {}` in `src/app/globals.css`
- Canvas: `#0A0A0B` base
- Accents: Violet `#7C3AED`, Cyan `#06B6D4`, Emerald `#10B981`
- Glassmorphism: `.glass`, `.glass-card` utility classes (backdrop-filter + subtle borders)
- Typography: Inter (via next/font) — `font-sans`, `font-display`

## Architecture
```
src/
├── app/
│   ├── globals.css    ← All design tokens + utilities
│   ├── layout.tsx     ← Root layout, fonts, SEO metadata
│   └── page.tsx       ← Main page (composes all sections)
├── components/
│   ├── ui/            ← Reusable: Button, Navigation
│   └── sections/      ← Hero, Problem, Features, Pricing, FAQ, CTA, Footer
└── lib/
    └── utils.ts       ← cn() utility
```

## Key Conventions
- New sections: add to `src/components/sections/`, import in `page.tsx`
- Client components for animation: `"use client"` directive
- All pages statically generated (SSG) by default
- SEO metadata: set in `layout.tsx` — Open Graph, Twitter Cards, JSON-LD, robots

## Commands
- `npm run dev` — start dev server
- `npm run build` — production build (must pass)
- `npm run lint` — ESLint
- `npm start` — serve production build
