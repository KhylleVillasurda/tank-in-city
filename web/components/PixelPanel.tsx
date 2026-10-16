import type { ReactNode } from "react";
import "./pixel/panel.css";

/*
 * The coin as panel.css plays it: 1040ms over eight steps, so a frame is held
 * for 130ms. Keep the two in step — the gallery below freezes the animation at
 * whole multiples of this, and a wrong number here shows the wrong frame.
 */
export const COIN = { frames: 8, frameMs: 130 };

/**
 * An 8-bit panel: a bevelled frame in one of the pixel palettes.
 *
 * The whole look lives in components/pixel, so this file is markup and nothing
 * else. Which palette is in use is `data-theme` on an ancestor, not a prop —
 * an ancestor is where a theme belongs, and it means this panel needs no state
 * and ships no JavaScript. Recolouring the site is a file in that directory.
 */
export default function PixelPanel({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <section className="pixel-panel">
      <h2 className="pixel-panel__title">{title}</h2>
      <div className="pixel-panel__body">{children}</div>
    </section>
  );
}

/**
 * The coin the panel spins: a strip of eight frames stepped by CSS.
 *
 * `frame` freezes it on that one frame rather than playing it, which is how the
 * gallery on /ui shows all eight at once — one sheet, one rule, no script.
 */
export function PixelCoin({ frame }: { frame?: number }) {
  return (
    <span
      className="pixel-coin"
      role="img"
      aria-label="A spinning coin"
      // A frozen frame is one of eight pictures of the same coin, so it does not
      // need announcing eight times.
      aria-hidden={frame === undefined ? undefined : true}
      style={
        frame === undefined
          ? undefined
          : { animationDelay: `${-frame * COIN.frameMs}ms`, animationPlayState: "paused" }
      }
    />
  );
}

/** The insert-coin bar: eight widths, filling left to right and starting again. */
export function PixelBar() {
  return (
    <div className="pixel-bar">
      <div className="pixel-bar__fill" />
    </div>
  );
}
