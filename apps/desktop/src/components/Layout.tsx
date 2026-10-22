/**
 * Layout.tsx
 * ----------
 * The full-window desktop shell: sidebar,
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
      <div className="flex h-full">
        <Sidebar />

        <div className="flex min-w-0 flex-1 flex-col">
          <TopBar />
          <ConnectionBanner />
          <main ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
            <AnimatePresence mode="wait" initial={false}>
              <motion.div
                key={pathname}
                initial={{ opacity: 0, y: 3 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -2 }}
                transition={
                  reduceMotion
                    ? { duration: 0.1 }
                    : { duration: 0.18, ease: [0.22, 1, 0.36, 1] }
                }
                className="content-shell flex flex-col gap-5"
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
