import type { Metadata } from "next";
import PixelPanel, { COIN, PixelBar, PixelCoin } from "../../components/PixelPanel";
import PixelTheme from "../../components/PixelTheme";

/*
 * One page with the whole pixel UI on it: the panel, the eight roles each
 * palette fills, and every frame of the coin.
 *
 * It is a demo and a test bench at once. Playwright drives this page to check
 * that the frames step and that nothing on screen is a colour outside the
 * palette (docs/pixel-ui.md), and a person can do most of the same by eye —
 * the eight frames are laid out side by side rather than played.
 */

export const metadata: Metadata = {
  title: "Pixel UI — Tank In City",
  description: "An 8-bit panel in three palettes: Eulbink, Ammo-8 and Cryptic Ocean.",
};

/** The roles a theme fills, in the order the panel.css comment lists them. */
const ROLES = [
  "background",
  "shadow",
  "panel",
  "line",
  "accent",
  "highlight",
  "text",
  "flash",
] as const;

const FRAMES = Array.from({ length: COIN.frames }, (_, frame) => frame);

export default function PixelUiPage() {
  return (
    <PixelTheme>
      <PixelPanel title="Insert coin">
        <div className="flex flex-wrap items-center gap-6">
          <PixelCoin />
          <p>
            One credit a round. Press start to drop into the city.
          </p>
        </div>
        <div className="mt-4">
          <PixelBar />
        </div>
      </PixelPanel>

      <PixelPanel title="The roles">
        <ul className="grid grid-cols-2 gap-4 sm:grid-cols-4">
          {ROLES.map((role) => (
            <li key={role}>
              {/* The chip is the role: its colour is whatever the theme says. */}
              <div className="h-12" style={{ background: `var(--pixel-${role})` }} />
              <span className="pixel-panel__note">{role}</span>
            </li>
          ))}
        </ul>
      </PixelPanel>

      <PixelPanel title="Every frame of the coin">
        <ul className="flex flex-wrap gap-4">
          {FRAMES.map((frame) => (
            <li key={frame}>
              <PixelCoin frame={frame} />
              <span className="pixel-panel__note">{frame}</span>
            </li>
          ))}
        </ul>
      </PixelPanel>
    </PixelTheme>
  );
}
