/**
 * Sound, worked out by comparing one frame to the one before it.
 *
 * There is no engine change behind any of this: the browser already receives a
 * snapshot every frame, so every event here is the difference between two of
 * them (docs/audio.md).
 *
 * Browsers refuse to make noise until the player has interacted with the page,
 * so the context is created suspended and resumed on the first key press or
 * click. Nothing is heard before that, which is why a run that starts without a
 * gesture is silent.
 */

import type { BulletView, Constants, Snapshot } from "./wasm";

const FILES = {
  shoot: "/audio/shoot.ogg",
  explosion: "/audio/explosion.ogg",
  stageClear: "/audio/level_cleared.ogg",
  music: "/audio/Battle_by_PerituneMaterial_8bitRPG.ogg",
  // The two tank_moving files are different sounds, not duplicates. Swap this
  // line to hear the other one.
  engine: "/audio/tank_moving_by_kenny.ogg",
} as const;

type Name = keyof typeof FILES;

/** Music sits under the effects rather than next to them. */
const MUSIC_GAIN = 0.6;
/** The engine is a drone; it has to be a background noise, not a sound effect. */
const ENGINE_GAIN = 0.3;
/** Frames the engine keeps rumbling after the tank stops, so taps do not stutter. */
const ENGINE_TAIL_FRAMES = 8;
/** Short ramps, so a loop that starts or stops does not click. */
const RAMP_SECONDS = 0.05;

export class Sounds {
  // Written out rather than a constructor parameter property: node's built-in
  // TypeScript stripping cannot handle those, and scripts/check-audio.mjs
  // imports this file directly. Keep it expanded.
  private consts: Constants;
  private ctx: AudioContext | null = null;
  private buffers = new Map<Name, AudioBuffer>();
  private music: AudioBufferSourceNode | null = null;
  private engine: { src: AudioBufferSourceNode; gain: GainNode } | null = null;
  private engineTail = 0;
  private prev: Snapshot | null = null;

  constructor(consts: Constants) {
    this.consts = consts;
  }

  /**
   * Create the audio context and start decoding. Safe to call before the player
   * has touched anything; the context simply stays suspended until they do.
   */
  start() {
    if (this.ctx) return;
    const ctx = new AudioContext();
    this.ctx = ctx;

    for (const name of Object.keys(FILES) as Name[]) {
      void fetch(FILES[name])
        .then((r) => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(String(r.status)))))
        .then((b) => ctx.decodeAudioData(b))
        .then((buffer) => this.buffers.set(name, buffer))
        // A missing or unplayable file means one silent sound, not a broken game.
        .catch(() => {});
    }

    // Any key press or click is the gesture the browser wants. Whichever
    // arrives first resumes the context; the other listener is harmless.
    const unlock = () => void ctx.resume().catch(() => {});
    window.addEventListener("keydown", unlock, { once: true });
    window.addEventListener("pointerdown", unlock, { once: true });
  }

  /** Play whatever the difference from the previous frame implies. */
  update(next: Snapshot) {
    const prev = this.prev;
    this.prev = next;

    const ctx = this.ctx;
    if (!ctx || ctx.state !== "running") return;

    if (!this.music) this.startMusic();
    if (!prev) return;

    const { statePlaying, stateStageClear } = this.consts;

    // Someone fired. Counted per side: the bullet list is compacted, so a slot
    // number says nothing about who owns it. A shot only happens when a slot is
    // free, so the count for that side always goes up when one is fired.
    // Player 0 is the local one; a match on the server is not this client's.
    const team = next.players[0]?.team ?? 0;
    if (countSide(next.bullets, team) > countSide(prev.bullets, team)) {
      this.play("shoot");
    }
    if (otherSide(next) > otherSide(prev)) this.play("shoot", 0.75);

    // A kill is the only thing that scores. The three ways to lose a tank all
    // share the one explosion, pitched apart.
    if (totalScore(next) > totalScore(prev)) this.play("explosion");
    if (totalLives(next) < totalLives(prev)) this.play("explosion", 0.7, 0.8);
    if (prev.baseAlive && !next.baseAlive) this.play("explosion", 0.5);

    if (next.state === stateStageClear && prev.state !== stateStageClear) {
      this.play("stageClear");
    }

    // The engine follows the wheels, with a short tail so slowing for a corner
    // does not stutter. Nothing moves while paused or off-stage, so it stops.
    const player = next.players[0];
    const was = prev.players[0];
    const moved = !!player && !!was && (player.x !== was.x || player.y !== was.y);
    if (next.state === statePlaying && player?.alive && moved) {
      this.engineTail = ENGINE_TAIL_FRAMES;
    } else if (this.engineTail > 0) this.engineTail--;

    if (this.engineTail > 0) this.startEngine();
    else this.stopEngine();
  }

  /** Silence everything and release the audio device. */
  stop() {
    this.music?.stop();
    this.music = null;
    this.stopEngine();
    void this.ctx?.close().catch(() => {});
    this.ctx = null;
    this.buffers.clear();
  }

  private play(name: Name, rate = 1, gain = 1) {
    const ctx = this.ctx;
    const buffer = this.buffers.get(name);
    if (!ctx || !buffer) return;

    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.playbackRate.value = rate;

    const volume = ctx.createGain();
    volume.gain.value = gain;

    src.connect(volume);
    volume.connect(ctx.destination);
    src.start();
  }

  private startMusic() {
    const ctx = this.ctx;
    const buffer = this.buffers.get("music");
    if (!ctx || !buffer || this.music) return;

    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.loop = true;

    const volume = ctx.createGain();
    volume.gain.value = MUSIC_GAIN;

    src.connect(volume);
    volume.connect(ctx.destination);
    src.start();
    this.music = src;
  }

  private startEngine() {
    const ctx = this.ctx;
    const buffer = this.buffers.get("engine");
    if (!ctx || !buffer || this.engine) return;

    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.loop = true;

    const gain = ctx.createGain();
    gain.gain.value = 0;

    src.connect(gain);
    gain.connect(ctx.destination);
    src.start();
    gain.gain.linearRampToValueAtTime(ENGINE_GAIN, ctx.currentTime + RAMP_SECONDS);
    this.engine = { src, gain };
  }

  private stopEngine() {
    const ctx = this.ctx;
    if (!this.engine || !ctx) return;

    const { src, gain } = this.engine;
    this.engine = null;
    gain.gain.linearRampToValueAtTime(0, ctx.currentTime + RAMP_SECONDS);
    src.stop(ctx.currentTime + RAMP_SECONDS * 1.2);
  }
}

/** Bullets in flight belonging to one side. */
function countSide(bullets: BulletView[], team: number): number {
  let n = 0;
  for (const b of bullets) if (b.team === team) n++;
  return n;
}

/** Bullets in flight from every side but player 0's own. */
function otherSide(s: Snapshot): number {
  return s.bullets.length - countSide(s.bullets, s.players[0]?.team ?? 0);
}

/** Every player's score added up. Single Player has just the one. */
function totalScore(s: Snapshot): number {
  let n = 0;
  for (const p of s.players) n += p.score;
  return n;
}

/** Every player's remaining lives added up. */
function totalLives(s: Snapshot): number {
  let n = 0;
  for (const p of s.players) n += p.lives;
  return n;
}
