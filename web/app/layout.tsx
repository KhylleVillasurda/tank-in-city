import type { Metadata } from "next";
import localFont from "next/font/local";
import "./globals.css";

/**
 * Press Start 2P, vendored in app/fonts with its OFL licence. Vendored rather
 * than pulled from Google at build time so a build never needs the network.
 */
const pixel = localFont({
  src: "./fonts/press-start-2p.ttf",
  variable: "--font-pixel-face",
  display: "swap",
  // No Arial metrics stand-in in the stack: a file that fails to load should
  // land on the site's own monospace face, not on a size-adjusted Arial.
  adjustFontFallback: false,
});

export const metadata: Metadata = {
  title: "Tank In City",
  description: "Top-down tank battles. Original rules, original art.",
};

export default function RootLayout({
  children,
}: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en">
      <body className={`${pixel.variable} crt min-h-dvh antialiased`}>
        {children}
      </body>
    </html>
  );
}
