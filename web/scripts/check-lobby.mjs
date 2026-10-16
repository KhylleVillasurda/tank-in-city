// Walks the lobby protocol (plan section 9) against a running game server.
//
//   cd server && cargo run           # one terminal
//   cd web && npm run check-lobby    # another
//
// The server is not part of the web build, so this is a separate check, the same
// way check-wasm.mjs and check-audio.mjs are. It answers the question a browser
// cannot answer on its own: is the socket working?
//
// Point it elsewhere with LOBBY_URL, which is what a tunnel needs:
//   LOBBY_URL=wss://tank-ws.example.com/ws npm run check-lobby

const url = process.env.LOBBY_URL ?? "ws://localhost:8080/ws";
const TIMEOUT = 5000;

function connect(url) {
  const socket = new WebSocket(url);
  const waiting = [];
  const arrived = [];

  socket.addEventListener("message", (event) => {
    const msg = JSON.parse(event.data);
    const i = waiting.findIndex((w) => w.type === msg.type);
    if (i === -1) arrived.push(msg);
    else waiting.splice(i, 1)[0].resolve(msg);
  });

  const open = () =>
    new Promise((resolve, reject) => {
      socket.addEventListener("open", resolve);
      socket.addEventListener("error", () =>
        reject(new Error(`could not reach ${url} — is the server running?`)),
      );
    });

  /** The next message of `type`, or a rejection after the timeout. */
  const next = (type) => {
    const i = arrived.findIndex((m) => m.type === type);
    if (i !== -1) return Promise.resolve(arrived.splice(i, 1)[0]);
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

  /** The next message of `type` that also satisfies `ok`. Stale ones are dropped. */
  const until = async (type, ok) => {
    for (;;) {
      const msg = await next(type);
      if (ok(msg)) return msg;
    }
  };

  return { socket, open, until, send: (msg) => socket.send(JSON.stringify(msg)) };
}

function check(ok, what) {
  if (!ok) throw new Error(what);
}

const host = connect(url);
const guest = connect(url);

try {
  await Promise.all([host.open(), guest.open()]);
  console.log(`connected to ${url}`);

  host.send({
    type: "create_room",
    name: "check-host",
    mode: "1v1",
    friendly_fire: false,
    public: true,
    // The page sends this as `null` rather than leaving it out, so the check
    // sends it the same way. A blank box is not a password.
    password: null,
  });
  const created = await host.until("room_state", (m) => m.players.length === 1);
  const code = created.code;
  check(/^[A-Z0-9]{5}$/.test(code), `room code ${code} is not five readable characters`);
  check(created.host === "check-host", `the host is ${created.host}`);
  check(created.status === "waiting", `a fresh room is ${created.status}`);
  console.log(`room ${code} created by ${created.host}`);

  guest.send({ type: "join_room", name: "check-guest", code, password: null });
  const joined = await guest.until("room_state", (m) => m.players.length === 2);
  check(joined.players[1].team === "b", `the guest is on side ${joined.players[1].team}`);
  console.log(`guest joined: ${joined.players.map((p) => `${p.name} (${p.team})`).join(", ")}`);

  guest.send({ type: "set_ready", ready: true });
  host.send({ type: "set_ready", ready: true });
  await host.until("room_state", (m) => m.players.every((p) => p.ready));
  console.log("both players ready");

  host.send({ type: "start_match" });
  const started = await host.until("room_state", (m) => m.status === "playing");
  check(started.players.length === 2, "the match started with the wrong roster");
  console.log("match started");

  guest.send({ type: "leave_room" });
  host.send({ type: "leave_room" });
  console.log("lobby ok: create, join, ready, start, leave");
} catch (error) {
  console.error(`lobby check failed: ${error.message}`);
  process.exitCode = 1;
} finally {
  host.socket.close();
  guest.socket.close();
}
