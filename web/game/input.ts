/**
 * Keyboard to engine input bits.
 *
 * Arrows or WASD to drive, Space to fire, P to pause, R to restart. Enter or a
 * click moves on from a stage-clear screen. The most recently pressed direction
 * wins, so holding left + up drives in whichever you hit last instead of
 * fighting over the tile grid.
 */

import type { Constants } from "./wasm";

const DIRECTIONS: Record<string, "inputUp" | "inputDown" | "inputLeft" | "inputRight"> = {
  arrowup: "inputUp",
  w: "inputUp",
  arrowdown: "inputDown",
  s: "inputDown",
  arrowleft: "inputLeft",
  a: "inputLeft",
  arrowright: "inputRight",
  d: "inputRight",
};

const HELD_KEYS = new Set(["arrowup", "arrowdown", "arrowleft", "arrowright", " ", "spacebar"]);

export class Keyboard {
  /** Direction keys in press order; the last one is the one being driven. */
  private pressed: Array<keyof Constants> = [];
  private fire = false;
  /** Set by a click or a fresh key press; cleared once it has been sent. */
  private confirm = false;
  private once = new Set<string>();

  constructor(private consts: Constants) {
    window.addEventListener("keydown", this.onDown);
    window.addEventListener("keyup", this.onUp);
    window.addEventListener("blur", this.onBlur);
  }

  dispose() {
    window.removeEventListener("keydown", this.onDown);
    window.removeEventListener("keyup", this.onUp);
    window.removeEventListener("blur", this.onBlur);
  }

  /** Bits for one simulation step. */
  bits(): number {
    let bits = this.fire ? this.consts.inputFire : 0;
    if (this.confirm) {
      bits |= this.consts.inputConfirm;
      this.confirm = false;
    }
    const dir = this.pressed[this.pressed.length - 1];
    if (dir) bits |= this.consts[dir];
    return bits;
  }

  /** The player clicked, asking to leave a stage-clear screen. */
  clicked() {
    this.confirm = true;
  }

  /** True once per key press, for one-shot keys like R. */
  takePress(key: string): boolean {
    if (!this.once.has(key)) return false;
    this.once.delete(key);
    return true;
  }

  private onDown = (e: KeyboardEvent) => {
    const key = e.key.toLowerCase();
    if (HELD_KEYS.has(key)) e.preventDefault();

    // A held key repeats. Only a fresh press confirms, or holding fire through
    // the end of a stage would skip the stage-clear screen instantly.
    if (e.repeat) return;

    // Enter confirms. Deliberately *not* fire: a player still mashing the fire
    // key when the last enemy dies would skip past the screen and the sound.
    if (key === "enter") {
      this.confirm = true;
      return;
    }

    const dir = DIRECTIONS[key];
    if (dir) {
      this.pressed = this.pressed.filter((d) => d !== dir);
      this.pressed.push(dir);
      return;
    }
    if (key === " ") this.fire = true;
    else this.once.add(key);
  };

  private onUp = (e: KeyboardEvent) => {
    const key = e.key.toLowerCase();
    const dir = DIRECTIONS[key];
    if (dir) {
      this.pressed = this.pressed.filter((d) => d !== dir);
      return;
    }
    if (key === " ") this.fire = false;
  };

  // Losing focus mid-drive would otherwise leave a key stuck down.
  private onBlur = () => {
    this.pressed = [];
    this.fire = false;
  };
}
