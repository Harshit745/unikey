"use client";

import { useState, useEffect } from "react";
import { cn } from "@/lib/utils";

const NAV_LINKS = [
  { label: "Principles", href: "#principles" },
  { label: "Pricing", href: "#pricing" },
  { label: "FAQ", href: "#faq" },
];

export function Navigation() {
  const [scrolled, setScrolled] = useState(false);

  useEffect(() => {
    const handler = () => setScrolled(window.scrollY > 40);
    window.addEventListener("scroll", handler, { passive: true });
    return () => window.removeEventListener("scroll", handler);
  }, []);

  return (
    <header
      className={cn(
        "fixed top-0 left-0 right-0 z-50 transition-all duration-500",
        scrolled ? "glass-thin border-b border-white/[0.03]" : ""
      )}
    >
      <div className="mx-auto max-w-6xl px-6 sm:px-8 lg:px-10">
        <div className="flex h-14 items-center justify-between">
          <a href="/" className="tracking-tight text-sm font-medium text-text-primary">
            UniKey
          </a>

          <nav className="flex items-center gap-8">
            {NAV_LINKS.map((link) => (
              <a
                key={link.href}
                href={link.href}
                className="text-sm text-text-tertiary hover:text-text-primary transition-colors duration-200"
              >
                {link.label}
              </a>
            ))}
            <a
              href="#"
              className="text-sm font-medium text-text-primary hover:opacity-70 transition-opacity"
            >
              Download
            </a>
          </nav>
        </div>
      </div>
    </header>
  );
}
