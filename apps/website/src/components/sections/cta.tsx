"use client";

import { useRef } from "react";
import { motion, useInView } from "framer-motion";

export function CTA() {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-80px" });

  return (
    <section ref={ref} className="py-32 px-6">
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={isInView ? { opacity: 1, y: 0 } : {}}
        transition={{ duration: 0.8 }}
        className="max-w-lg mx-auto text-center"
      >
        <h2 className="text-3xl sm:text-4xl font-medium tracking-tight text-text-primary">
          Your clipboard everywhere.
        </h2>
        <p className="mt-3 text-sm text-text-tertiary">
          No cloud. No accounts. No configuration.
        </p>
        <div className="mt-8">
          <a
            href="#"
            className="inline-flex items-center gap-2 h-11 px-5 rounded-[8px] bg-text-primary text-canvas text-sm font-medium hover:opacity-85 transition-opacity"
          >
            Download UniKey
            <span className="text-canvas/40">→</span>
          </a>
        </div>
      </motion.div>
    </section>
  );
}
