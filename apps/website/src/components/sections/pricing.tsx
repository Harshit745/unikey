"use client";

import { useRef } from "react";
import { motion, useInView } from "framer-motion";
import { Check } from "lucide-react";

const features = [
  "Unlimited devices",
  "Universal Clipboard",
  "Universal Keyboard",
  "Universal Mouse",
  "Drag & Drop files",
  "Clipboard history",
  "End-to-end encryption",
  "LAN-only mode",
];

export function Pricing() {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-80px" });

  return (
    <section id="pricing" ref={ref} className="py-32 px-6">
      <div className="max-w-md mx-auto text-center">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.8 }}
        >
          <span className="text-xs tracking-[0.15em] text-text-quaternary font-medium uppercase">
            Pricing
          </span>
          <h2 className="mt-8 text-4xl sm:text-5xl font-medium tracking-tight text-text-primary">
            Free during beta.
          </h2>
          <p className="mt-3 text-sm text-text-tertiary">
            Everything included. No trial. No credit card.
          </p>
        </motion.div>

        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.8, delay: 0.2 }}
          className="mt-10 bg-surface rounded-[12px] p-8 text-left"
        >
          <div className="flex items-baseline gap-1.5 mb-6">
            <span className="text-5xl font-light text-text-primary">$0</span>
            <span className="text-sm text-text-tertiary">forever</span>
          </div>

          <a
            href="#"
            className="inline-flex items-center justify-center gap-2 w-full h-11 rounded-[8px] bg-text-primary text-canvas text-sm font-medium hover:opacity-85 transition-opacity mb-8"
          >
            Download UniKey
          </a>

          <div className="grid grid-cols-2 gap-x-6 gap-y-3">
            {features.map((f) => (
              <div key={f} className="flex items-center gap-2.5 text-sm text-text-secondary">
                <Check className="w-3.5 h-3.5 text-accent shrink-0" />
                {f}
              </div>
            ))}
          </div>
        </motion.div>
      </div>
    </section>
  );
}
