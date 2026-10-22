import type { Metadata, Viewport } from "next";
import { Inter, Geist_Mono } from "next/font/google";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
  display: "swap",
  preload: true,
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
  display: "swap",
  preload: true,
});

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  themeColor: "#0A0A0B",
};

export const metadata: Metadata = {
  title: {
    template: "%s — UniKey",
    default: "UniKey — The Operating Layer for Every Device",
  },
  description:
    "Universal Keyboard, Clipboard, and File Transfer across all your devices. LAN-first, private, end-to-end encrypted. One seamless workspace.",
  keywords: [
    "UniKey",
    "universal clipboard",
    "cross-platform keyboard",
    "file transfer",
    "LAN",
    "privacy",
  ],
  authors: [{ name: "UniKey" }],
  metadataBase: new URL("https://unikey.dev"),
  openGraph: {
    type: "website",
    locale: "en_US",
    siteName: "UniKey",
    title: "UniKey — The Operating Layer for Every Device",
    description:
      "Universal Keyboard, Clipboard, and File Transfer across all your devices.",
  },
  twitter: {
    card: "summary_large_image",
    title: "UniKey — The Operating Layer for Every Device",
    description:
      "Universal Keyboard, Clipboard, and File Transfer across all your devices.",
  },
  robots: {
    index: true,
    follow: true,
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${inter.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="min-h-full bg-canvas text-text-primary font-sans">
        {children}
      </body>
    </html>
  );
}
