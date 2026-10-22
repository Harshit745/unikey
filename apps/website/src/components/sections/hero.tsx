"use client";

import { useState, useRef } from "react";
import { motion, useScroll, useTransform } from "framer-motion";

const spring = { type: "spring", stiffness: 250, damping: 25, mass: 1 };

export function Hero() {
  const [text, setText] = useState("");
  const [clipboard, setClipboard] = useState("");
  const [showCopied, setShowCopied] = useState(false);
  const ref = useRef<HTMLDivElement>(null);
  const { scrollYProgress } = useScroll({ target: ref, offset: ["start start", "end start"] });
  const opacity = useTransform(scrollYProgress, [0, 0.6], [1, 0]);
  const y = useTransform(scrollYProgress, [0, 0.6], [0, 60]);

  const handleCopy = () => {
    if (!text.trim()) return;
    setClipboard(text);
    setShowCopied(true);
    setTimeout(() => setShowCopied(false), 2000);
  };

  return (
    <section ref={ref} className="min-h-screen flex items-center justify-center px-6">
      <motion.div style={{ opacity, y }} className="max-w-3xl mx-auto w-full text-center">

        {/* Editorial headline */}
        <motion.h1
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 1.2 }}
          className="text-5xl sm:text-6xl md:text-7xl lg:text-8xl font-medium tracking-tight leading-[0.95] text-balance"
        >
          Your clipboard
          <br />
          <span className="text-text-tertiary">everywhere.</span>
        </motion.h1>

        <motion.p
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.3 }}
          className="mt-6 text-base sm:text-lg text-text-tertiary max-w-sm mx-auto leading-relaxed"
        >
          Copy on one device. Paste on any other.
          No cloud. No accounts.
        </motion.p>

        {/* Download */}
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.8, delay: 0.5 }}
          className="mt-8"
        >
          <a
            href="#"
            className="inline-flex items-center gap-2 h-11 px-5 rounded-[8px] bg-text-primary text-canvas text-sm font-medium hover:opacity-85 transition-opacity"
          >
            Download UniKey
            <span className="text-canvas/60">macOS · Windows · Linux</span>
          </a>
        </motion.div>

        {/* ── Live Demo ─────────────────────────────────── */}
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 1, delay: 0.8 }}
          className="mt-20 max-w-lg mx-auto"
        >
          {/* Sender */}
          <div className="text-left mb-8">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-1.5 h-1.5 rounded-full bg-status-online" />
              <span className="text-xs text-text-quaternary font-medium tracking-wide uppercase">
                MacBook Pro
              </span>
            </div>
            <div className="relative">
              <textarea
                value={text}
                onChange={(e) => setText(e.target.value)}
                placeholder="Type something to copy…"
                rows={2}
                className="w-full bg-surface-secondary rounded-[8px] px-4 py-3 text-sm text-text-primary placeholder:text-text-quaternary resize-none focus:outline-none focus:ring-1 focus:ring-accent/30 transition-shadow"
              />
              <button
                onClick={handleCopy}
                className="absolute right-2.5 bottom-2.5 h-7 px-2.5 rounded-[6px] bg-surface-tertiary hover:bg-surface text-xs text-text-secondary transition-colors"
              >
                {showCopied ? "Copied" : "Copy"}
              </button>
            </div>
          </div>

          {/* Divider — simple dash */}
          <div className="flex items-center gap-3 mb-8 text-text-quaternary">
            <div className="flex-1 h-px bg-white/[0.04]" />
            <span className="text-xs">{clipboard ? "Synced" : "LAN"}</span>
            <div className="flex-1 h-px bg-white/[0.04]" />
          </div>

          {/* Receiver */}
          <div className="text-left">
            <div className="flex items-center gap-2 mb-3">
              <div className="w-1.5 h-1.5 rounded-full bg-status-online" />
              <span className="text-xs text-text-quaternary font-medium tracking-wide uppercase">
                Windows Desktop
              </span>
            </div>
            <div className="w-full min-h-[44px] bg-surface-secondary rounded-[8px] px-4 py-3">
              {clipboard ? (
                <motion.p
                  key={clipboard}
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  className="text-sm text-text-primary"
                >
                  {clipboard}
                </motion.p>
              ) : (
                <p className="text-sm text-text-quaternary">Paste destination</p>
              )}
            </div>
          </div>
        </motion.div>

      </motion.div>
    </section>
  );
}
