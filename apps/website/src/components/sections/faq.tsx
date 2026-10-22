"use client";

import { useState, useRef } from "react";
import { motion, useInView, AnimatePresence } from "framer-motion";

const faqs = [
  {
    q: "How does UniKey work?",
    a: "UniKey creates a direct peer-to-peer connection between your devices over your local network. Clipboard, keyboard, and input flow directly — encrypted, instant, no cloud servers.",
  },
  {
    q: "Is it really encrypted?",
    a: "Yes. Every transmission uses AES-256-GCM. Keys are exchanged during the initial pairing handshake. Your data never touches a third-party server.",
  },
  {
    q: "Do I need an account?",
    a: "No. UniKey has no accounts, no sign-up, no cloud dependency. Devices discover each other on your local network. Pair once, and they remember each other.",
  },
];

export function FAQ() {
  const ref = useRef(null);
  const isInView = useInView(ref, { once: true, margin: "-80px" });
  const [open, setOpen] = useState<number | null>(null);

  return (
    <section id="faq" ref={ref} className="py-32 px-6">
      <div className="max-w-lg mx-auto">
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={isInView ? { opacity: 1, y: 0 } : {}}
          transition={{ duration: 0.8 }}
          className="text-center mb-12"
        >
          <span className="text-xs tracking-[0.15em] text-text-quaternary font-medium uppercase">
            FAQ
          </span>
        </motion.div>

        <div className="space-y-2">
          {faqs.map((faq, i) => (
            <motion.div
              key={i}
              initial={{ opacity: 0 }}
              animate={isInView ? { opacity: 1 } : {}}
              transition={{ duration: 0.5, delay: 0.1 * i }}
              className="border-b border-white/[0.04]"
            >
              <button
                onClick={() => setOpen(open === i ? null : i)}
                className="w-full flex items-center justify-between py-4 text-left"
              >
                <span className="text-sm text-text-primary">{faq.q}</span>
                <span className={`text-text-quaternary text-xs transition-transform duration-200 ${open === i ? "rotate-45" : ""}`}>
                  +
                </span>
              </button>
              <AnimatePresence>
                {open === i && (
                  <motion.div
                    initial={{ height: 0, opacity: 0 }}
                    animate={{ height: "auto", opacity: 1 }}
                    exit={{ height: 0, opacity: 0 }}
                    transition={{ duration: 0.2 }}
                    className="overflow-hidden"
                  >
                    <p className="pb-4 text-sm text-text-tertiary leading-relaxed">
                      {faq.a}
                    </p>
                  </motion.div>
                )}
              </AnimatePresence>
            </motion.div>
          ))}
        </div>
      </div>
    </section>
  );
}
