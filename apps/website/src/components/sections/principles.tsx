"use client";

import { useRef } from "react";
import { motion, useInView } from "framer-motion";

const principles = [
  {
    title: "Copy anywhere.\nPaste everywhere.",
    body: "The text, link, or image you copied is available on every device you own — instantly and securely, as if they shared one clipboard buffer.",
  },
  {
    title: "Zero setup.",
    body: "Install once. Devices on the same network find each other automatically. No accounts, no sign-up, no configuration.",
  },
  {
    title: "End-to-end encrypted.",
    body: "Every transmission is encrypted with AES-256. Keys are exchanged during pairing. Your data never touches a third party.",
  },
];

export function Principles() {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-80px" });

  return (
    <section id="principles" ref={ref} className="py-32 px-6">
      <div className="max-w-4xl mx-auto space-y-28">
        {principles.map((p, i) => (
          <motion.div
            key={p.title}
            initial={{ opacity: 0, y: 30 }}
            animate={isInView ? { opacity: 1, y: 0 } : {}}
            transition={{ duration: 0.8, delay: 0.15 * i }}
            className="grid sm:grid-cols-[1fr_1.5fr] gap-8 sm:gap-16"
          >
            <h2 className="text-2xl sm:text-3xl font-medium text-text-primary leading-tight whitespace-pre-line">
              {p.title}
            </h2>
            <p className="text-sm sm:text-base text-text-tertiary leading-relaxed max-w-md">
              {p.body}
            </p>
          </motion.div>
        ))}
      </div>
    </section>
  );
}
