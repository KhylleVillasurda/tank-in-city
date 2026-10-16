/**
 * The game loop: a fixed 60 Hz simulation driven by requestAnimationFrame.
 *
 * Rendering runs at the display's rate; the engine always advances in whole
 * `tickHz` steps, which is the whole point of the fixed timestep in plan
 * section 4. Pausing just stops feeding it steps.
 */

import { Sounds } from "./audio";
import { Keyboard } from "./input";
import { createRenderer, type Renderer } from "./render";
import {
  readSnapshot,
  readTiles,
  type Constants,
  type Core,
  type Snapshot,
} from "./wasm";

/** Longest catch-up burst after a stall, in milliseconds. */
const MAX_CATCHUP_MS = 250;

export class Game {
  private renderer: Renderer;
  private keyboard: Keyboard;
  private sounds: Sounds;
  private raf = 0;
  private acc = 0;
  private last = 0;
  private paused = false;
  /** The seed of the run in progress, so a report of one can be replayed. */
  private seed = 0;

  constructor(
    private core: Core,
    private consts: Constants,
    canvas: HTMLCanvasElement,
    private onFrame: (snapshot: Snapshot, paused: boolean, seed: number) => void,
  ) {
    this.renderer = createRenderer(canvas, consts);
    this.keyboard = new Keyboard(consts);
    this.sounds = new Sounds(consts);
    this.sounds.start();
  }

  start(seed = newSeed()) {
    this.seed = seed >>> 0;
    this.core.tc_init(this.seed);
    this.acc = 0;
    this.paused = false;
    this.last = performance.now();
    this.raf = requestAnimationFrame(this.frame);
  }

  stop() {
    cancelAnimationFrame(this.raf);
    this.keyboard.dispose();
    this.sounds.stop();
  }

  /** The player clicked the canvas: leave a stage-clear screen. */
  clicked() {
    this.keyboard.clicked();
  }

  /** The music switch on the game page. */
  setMusicOff(off: boolean) {
    this.sounds.setMusicOff(off);
  }

  private frame = (now: number) => {
    if (this.keyboard.takePress("p") || this.keyboard.takePress("escape")) {
      this.paused = !this.paused;
    }

    this.acc += Math.min(now - this.last, MAX_CATCHUP_MS);
    this.last = now;

    if (!this.paused) {
      const step = 1000 / this.consts.tickHz;
      while (this.acc >= step) {
        this.core.tc_step(this.keyboard.bits());
        this.acc -= step;
      }
    }

    const snapshot = readSnapshot(this.core, this.consts);
    this.sounds.update(snapshot);

    if (snapshot.state === this.consts.stateGameOver && this.keyboard.takePress("r")) {
      this.start();
      return;
    }

    this.renderer.draw(snapshot, readTiles(this.core, this.consts));
    this.onFrame(snapshot, this.paused, this.seed);
    this.raf = requestAnimationFrame(this.frame);
  };
}

function newSeed(): number {
  return (Math.random() * 0xffffffff) >>> 0;
}
