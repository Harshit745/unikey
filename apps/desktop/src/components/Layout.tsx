/**
 * Layout.tsx
 * ----------
 * The full-window glass shell: aurora backdrop, collapsible sidebar,
 * top bar, offline banner, and the routed content column. Route changes
 * cross-fade via AnimatePresence keyed on the pathname. The sidebar
 * persists across routes so the nav pill keeps its layoutId.
 */

import React, { useEffect, useRef } from 'react';
import { Outlet, useLocation } from 'react-router-dom';
import { AnimatePresence, motion } from 'motion/react';
import { Sidebar } from './Sidebar';
import { TopBar } from './TopBar';
import { ConnectionBanner } from './ConnectionBanner';
import { useAppearance } from '@/context/AppearanceContext';

export const Layout: React.FC = () => {
  const { pathname } = useLocation();
  const { reduceMotion } = useAppearance();
  const scrollRef = useRef<HTMLDivElement>(null);

  // Reset scroll on navigation.
  useEffect(() => {
    scrollRef.current?.scrollTo({ top: 0 });
  }, [pathname]);

  return (
    <div className="stage">
      {/* Aurora — the "content scrolling under" the glass (§12) */}
      <div className="blobs" aria-hidden="true">
        <span className="blob b1" /><span className="blob b2" /><span className="blob b3" />
        <span className="blob b4" /><span className="blob b5" />
      </div>

      <div className="relative z-10 flex h-full">
        <Sidebar />

        <div className="flex min-w-0 flex-1 flex-col">
          <TopBar />
          <ConnectionBanner />
          <main ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={pathname}
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -6 }}
                transition={
                  reduceMotion
                    ? { duration: 0.1 }
                    : { duration: 0.28, ease: [0.22, 1, 0.36, 1] }
                }
                className="mx-auto flex max-w-5xl flex-col gap-6 px-6 py-6"
              >
                <Outlet />
              </motion.div>
            </AnimatePresence>
          </main>
        </div>
      </div>
    </div>
  );
};
