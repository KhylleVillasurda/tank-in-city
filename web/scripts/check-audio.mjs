// Headless check for the sound triggers in game/audio.ts.
// Run: npm run check-audio
//
// There is no speaker here, so this cannot say whether a sound is *right*. It
// says each one fires when it should, and does not fire when the browser has not
// been unlocked yet. The audio device is replaced with a stub that records what
// was asked to play.
import { readFile } from "node:fs/promises";

import { Sounds } from "../game/audio.ts";
import { loadCore, readSnapshot } from "../game/wasm.ts";

const played = [];

class FakeSource {
  constructor() {
    this.buffer = null;
    this.loop = false;
    this.playbackRate = { value: 1 };
  }
  connect() {}
  start() {
    played.push({ name: this.buffer?.name ?? "?", rate: this.playbackRate.value });
  }
  stop() {}
}

class FakeGain {
  constructor() {
    this.gain = { value: 1, linearRampToValueAtTime() {} };
  }
  connect() {}
}

class FakeContext {
  constructor() {
    this.state = "suspended";
    this.currentTime = 0;
    this.destination = {};
  }
  async resume() {
    this.state = "running";
  }
  async close() {
    this.state = "closed";
  }
  // The stub hands back the URL it fetched, which is how each buffer keeps its
  // name through decode.
  async decodeAudioData(bytes) {
    return { name: new TextDecoder().decode(bytes) };
  }
  createBufferSource() {
    return new FakeSource();
  }
  createGain() {
    return new FakeGain();
  }
}

globalThis.AudioContext = FakeContext;
globalThis.window = { addEventListener() {} };
globalThis.fetch = async (url) => {
  if (String(url).endsWith(".wasm")) {
    const bytes = await readFile(new URL("../public/game_core.wasm", import.meta.url));
    return { ok: true, arrayBuffer: async () => bytes };
  }
  return { ok: true, arrayBuffer: async () => new TextEncoder().encode(String(url)) };
};

function assert(ok, what) {
  if (!ok) throw new Error(`FAIL: ${what}`);
}

const { core, consts } = await loadCore();
const sounds = new Sounds(consts);
sounds.start();

/** Every player's score and lives added up. Single Player is one player. */
const scoreOf = (s) => s.players.reduce((n, p) => n + p.score, 0);
const livesOf = (s) => s.players.reduce((n, p) => n + p.lives, 0);

// --- Nothing plays before the player has interacted ---
const tick = () => readSnapshot(core, consts);
core.tc_init(11);
sounds.update(tick());
for (let i = 0; i < 120; i++) {
  core.tc_step(consts.inputRight | consts.inputFire);
  sounds.update(tick());
}
assert(played.length === 0, "a locked (suspended) context stays silent");

// --- A real gesture unlocks it ---
await sounds.ctx.resume();
assert(sounds.ctx.state === "running", "resume unlocks the context");
await new Promise((r) => setTimeout(r, 0)); // the decodes were fire-and-forget
assert(sounds.buffers.size === 5, `all five files decoded, got ${sounds.buffers.size}`);

// --- Playing now makes noise, and the right noise ---
played.length = 0;
core.tc_init(7);
sounds.update(tick());
const music = "/audio/Battle_by_PerituneMaterial_8bitRPG.ogg";
assert(played.filter((p) => p.name === music).length === 1, "music starts once");
sounds.update(tick());
assert(played.filter((p) => p.name === music).length === 1, "and not again next frame");

played.length = 0;
for (let i = 0; i < 400; i++) {
  core.tc_step(consts.inputFire);
  sounds.update(tick());
}
const shots = played.filter((p) => p.name === "/audio/shoot.ogg");
assert(shots.length > 0, "firing plays the shot");
assert(
  shots.some((s) => s.rate === 1),
  "the player's shot is not pitched",
);

// --- The engine loop follows the wheels ---
played.length = 0;
for (let i = 0; i < 200; i++) {
  core.tc_step(consts.inputUp);
  sounds.update(tick());
}
assert(
  played.some((p) => p.name === "/audio/tank_moving_by_kenny.ogg"),
  "driving starts the engine loop",
);

played.length = 0;
for (let i = 0; i < 40; i++) {
  core.tc_step(0);
  sounds.update(tick());
}
assert(
  !played.some((p) => p.name === "/audio/tank_moving_by_kenny.ogg"),
  "standing still lets the engine loop die away",
);

// --- A kill is an explosion, and the run's score going up is how we know ---
// A fresh run, not the one above: that player is long dead by now, and a dead
// run scores nothing.
core.tc_init(1);
sounds.update(tick());
played.length = 0;
let kills = 0;
const scoreAt = () => scoreOf(tick());
let last = scoreAt();
for (let i = 0; i < 9000 && kills === 0; i++) {
  core.tc_step(i % 160 < 40 ? consts.inputUp | consts.inputFire
    : i % 160 < 80 ? consts.inputRight | consts.inputFire
    : i % 160 < 120 ? consts.inputLeft
    : consts.inputLeft | consts.inputFire);
  sounds.update(tick());
  const now = scoreAt();
  if (now > last) kills++;
  last = now;
}
assert(kills > 0, "the scripted run killed something");
assert(
  played.some((p) => p.name === "/audio/explosion.ogg" && p.rate === 1),
  "a kill plays the explosion",
);
assert(
  played.some((p) => p.name === "/audio/shoot.ogg" && p.rate === 0.75),
  "an enemy's shot is the same sound, pitched down",
);

// --- Losing things: the same explosion, pitched down ---
// Asserted against what the run actually did, rather than against a guess about
// which way an idle player dies.
core.tc_init(3);
sounds.update(tick());
played.length = 0;
let lastLives = livesOf(tick());
let lastBase = tick().baseAlive;
let lostLife = false;
let lostBase = false;
for (let i = 0; i < 60 * 300; i++) {
  core.tc_step(0);
  const s = tick();
  sounds.update(s);
  if (livesOf(s) < lastLives) lostLife = true;
  if (lastBase && !s.baseAlive) lostBase = true;
  lastLives = livesOf(s);
  lastBase = s.baseAlive;
}
assert(lostLife || lostBase, "the idle player lost a life or the base");
if (lostLife) {
  assert(
    played.some((p) => p.name === "/audio/explosion.ogg" && p.rate === 0.7),
    "a lost life plays the explosion pitched down",
  );
}
if (lostBase) {
  assert(
    played.some((p) => p.name === "/audio/explosion.ogg" && p.rate === 0.5),
    "losing the base plays the lowest pitch of the same explosion",
  );
}

// --- Stage clear cannot be reached from here: it needs 20 kills. The trigger
// is exercised with a synthetic frame instead. ---
played.length = 0;
sounds.update({ ...tick(), state: consts.statePlaying });
sounds.update({ ...tick(), state: consts.stateStageClear });
assert(
  played.some((p) => p.name === "/audio/level_cleared.ogg"),
  "stage clear plays the jingle",
);
const jingles = played.length;
sounds.update({ ...tick(), state: consts.stateStageClear });
assert(played.length === jingles, "the jingle does not retrigger while the screen is up");

sounds.stop();
console.log(`audio ok: ${played.length} sounds in the last stretch, all five files decoded`);
