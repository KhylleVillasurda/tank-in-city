"use client";

import { useEffect, useState, type ReactNode } from "react";

/*
 * The palettes the pixel panel ships with.
 *
 * An id is the name of a file in components/pixel, which is also what goes in
 * `data-theme`. Adding a palette is that file, then a line here so the switch
 * offers it — the only place a theme is listed twice.
 */
const THEMES = [
  { id: "eulbink", label: "Eulbink" },
  { id: "ammo-8", label: "Ammo-8" },
  { id: "cryptic-ocean", label: "Cryptic Ocean" },
] as const;

type Theme = (typeof THEMES)[number]["id"];

const isTheme = (value: string | null): value is Theme =>
  THEMES.some((theme) => theme.id === value);

/** Kept beside the music switch's own key, and for the same reason. */
const THEME_KEY = "tank-in-city.pixel-theme";

/**
 * Holds the chosen palette and puts it on everything inside: the buttons, and
 * the `data-theme` the panel's CSS looks for.
 *
 * The first paint is always the default palette, because the default is the one
 * the stylesheet already falls back to for a bare `.pixel` — so a page with this
 * panel on it is themed before any script runs, and the stored choice only
 * catches up. That also means no flash of the wrong palette.
 */
export default function PixelTheme({ children }: { children: ReactNode }) {
  const [theme, setTheme] = useState<Theme>("eulbink");

  useEffect(() => {
    const stored = localStorage.getItem(THEME_KEY);
    if (isTheme(stored)) setTheme(stored);
  }, []);

  const choose = (id: Theme) => {
    setTheme(id);
    localStorage.setItem(THEME_KEY, id);
  };

  return (
    <div className="pixel min-h-dvh p-8" data-theme={theme}>
      <div className="mx-auto flex w-full max-w-3xl flex-col gap-8">
        <div className="flex flex-wrap gap-2" role="group" aria-label="Palette">
          {THEMES.map((option) => (
            <button
              key={option.id}
              type="button"
              className="pixel-button"
              aria-pressed={option.id === theme}
              onClick={() => choose(option.id)}
            >
              {option.label}
            </button>
          ))}
        </div>
        {children}
      </div>
    </div>
  );
}
