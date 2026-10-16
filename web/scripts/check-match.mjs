// Drives a real match over the socket (plan sections 5 and 9).
//
//   cd server && cargo run           # one terminal
//   cd web && npm run check-match    # another
//
// This is the local stand-in for plan section 7's two-browser check: two real
// WebSocket clients, a real room, a real match. It answers what the Rust tests
// cannot — that a `map` and snapshots actually go out at the wire, and that an
// `input` moves the tank that sent it and no other.
//
// It needs `npm run wasm` (the engine's constants are read from the wasm, so no
// offset is copied here) and a server on `LOBBY_URL`. A tunnel works:
//   LOBBY_URL=wss://tank-ws.example.com/ws npm run check-match
//
// It cannot judge whether either tank *looked* right. That is the two-browser
// check, and it needs the user.

import { readFile } from "node:fs/promises";

import { CONSTANTS_FIELDS } from "../game/wasm.ts";

const url = process.env.LOBBY_URL ?? "ws://localhost:8080/ws";
const TIMEOUT = 5000;

/** The engine's constants, so the snapshot offsets are read, not guessed. */
async function constants() {
  let bytes;
  try {
    bytes = await readFile(new URL("../public/game_core.wasm", import.meta.url));
  } catch {
    throw new Error("no web/public/game_core.wasm — run npm run wasm first");
  }
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const raw = new Int32Array(
    instance.exports.memory.buffer,
    instance.exports.tc_constants_ptr(),
    CONSTANTS_FIELDS.length,
  );
  return Object.fromEntries(CONSTANTS_FIELDS.map((name, i) => [name, raw[i]]));
}

function client(url) {
  const socket = new WebSocket(url);
  /** Messages read while looking for another kind, so none is lost. */
  const pending = [];
  const waiting = [];

  socket.addEventListener("message", (event) => {
    const msg = JSON.parse(event.data);
    const i = waiting.findIndex((w) => w.type === msg.type);
    if (i === -1) pending.push(msg);
    else waiting.splice(i, 1)[0].resolve(msg);
  });

  const open = () =>
    new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve);
      socket.addEventListener("error", () =>
        reject(new Error(`could not reach ${url} — is the server running?`)),
      );
    });

  const next = (type) => {
    const i = pending.findIndex((m) => m.type === type);
    if (i !== -1) return Promise.resolve(pending.splice(i, 1)[0]);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error(`waited ${TIMEOUT}ms for a ${type}`)),
        TIMEOUT,
      );
      waiting.push({
        type,
        resolve: (msg) => {
          clearTimeout(timer);
          resolve(msg);
        },
      });
    });
  };

  /** The next message of `type` that `ok` accepts. Stale ones are dropped. */
  const until = async (type, ok) => {
    for (;;) {
      const msg = await next(type);
      if (ok(msg)) return msg;
    }
  };

  return { socket, open, next, until, send: (msg) => socket.send(JSON.stringify(msg)) };
}

function check(ok, what) {
  if (!ok) throw new Error(what);
}

const c = await constants();
const host = client(url);
const guest = client(url);

try {
  await Promise.all([host.open(), guest.open()]);
  console.log(`connected to ${url}`);

  host.send({
    type: "create_room",
    name: "match-host",
    mode: "1v1",
    friendly_fire: false,
    public: true,
    password: null,
  });
  const created = await host.until("room_state", (m) => m.players.length === 1);
  const code = created.code;

  guest.send({ type: "join_room", name: "match-guest", code, password: null });
  await guest.until("room_state", (m) => m.players.length === 2);
  await host.until("room_state", (m) => m.players.length === 2);

  host.send({ type: "set_ready", ready: true });
  guest.send({ type: "set_ready", ready: true });
  await host.until("room_state", (m) => m.players.every((p) => p.ready));

  host.send({ type: "start_match" });
  await host.until("room_state", (m) => m.status === "playing");
  await guest.until("room_state", (m) => m.status === "playing");
  console.log(`room ${code} started`);

  // The full map arrives once, when the match starts, to both players.
  const map = await host.next("map");
  check(
    map.tiles.length === c.mapW * c.mapH,
    `the map is ${map.tiles.length} tiles, not ${c.mapW * c.mapH}`,
  );
  check(map.tiles.includes(c.tileBrick), "the stage has no brick");
  check(map.tiles.includes(c.tileBase), "the stage has no base");
  await guest.next("map");
  console.log(`map received: ${map.tiles.length} tiles, with brick and a base`);

  // Then snapshots, the engine's flat state buffer, to both players.
  const first = await host.next("snapshot");
  check(
    first.data.length === c.snapLen,
    `a snapshot is ${first.data.length} ints, not ${c.snapLen}`,
  );
  await guest.next("snapshot");

  // A player's x in the snapshot. Players are in roster order, host first.
  const x = (snap, slot) => snap.data[c.snapPlayersOffset + slot * c.snapPlayer];
  const hostX = x(first, 0);
  const guestX = x(first, 1);

  // Only the host drives. The guest sends nothing, so its tank must stand
  // still — which is what proves each slot gets its own input.
  host.send({ type: "input", move: c.inputRight, fire: false, tick: 1 });
  let snap = first;
  for (let i = 0; i < 120 && x(snap, 0) === hostX; i++) {
    snap = await host.next("snapshot");
    check(x(snap, 1) === guestX, "the guest's tank moved without an input");
  }
  check(x(snap, 0) > hostX, `the host's tank never moved right from x=${hostX}`);
  console.log(`input moved the host's tank: x ${hostX} -> ${x(snap, 0)}`);

  guest.send({ type: "leave_room" });
  host.send({ type: "leave_room" });
  console.log("match ok: map, snapshots, and input moving only the sender");
} catch (error) {
  console.error(`match check failed: ${error.message}`);
  process.exitCode = 1;
} finally {
  host.socket.close();
  guest.socket.close();
}
