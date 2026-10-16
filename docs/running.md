# Running Tank In City

One page: how to build it, run it, check it, and put it on a public link. The
tunnel details, the security notes and the troubleshooting come from
`docs/sharing.md` — this file is the short version.

## Prerequisites

| Tool | For | Check |
|---|---|---|
| Rust + `wasm32-unknown-unknown` | the game (`game-core`) and the server | `rustup target add wasm32-unknown-unknown` |
| Node (20 or newer) | the Next.js front end and the check scripts | `node --version` |
| `cloudflared` | sharing a link, optional | `cloudflared --version` |

No `wasm-pack` and no `wasm-bindgen`: the wasm is a plain
`cargo build --target wasm32-unknown-unknown --release`.

## First time

```sh
rustup target add wasm32-unknown-unknown
cd web && npm install
```

## Everyday commands

| Command | What it starts | Where |
|---|---|---|
| `cd web && npm run dev` | the game, rebuilt wasm first | http://localhost:3000 |
| `cd web && npm run build && npm start` | the production build — the one to share | http://localhost:3000 |
| `cd server && cargo run` | the game server: `/ws` and `/health` | 127.0.0.1:8080 |

`predev` and `prebuild` rebuild the wasm, so the browser never runs a module
older than the Rust source. `npm run wasm` does that on its own.

Pages: `/` menu, `/menu`, `/game` (Single Player), `/lobby`, `/lobby?server=…`.

The server binds `127.0.0.1` only, so nothing reaches it but your own machine and
a tunnel. `PORT=9000 cargo run` moves it; `deploy/cloudflared.yml` expects 8080.

## What works today

- **Single Player** — fully. No server needed.
- **The lobby** — create, join by code, sides, ready, host transfer, start.
- **A match** — the server runs it: one `World` per room at 60 Hz, broadcasting
  `map` and `snapshot`. **There is no match page in the browser yet**, so a
  started match has nothing rendering it. That page is the next slice.

## Checks

```sh
cd game-core && cargo test            # 37 tests
cd game-core && cargo clippy --all-targets    # must be silent
cd server    && cargo test            # 44 tests
cd server    && cargo clippy --all-targets    # must be silent

cd web && npm run check               # the wasm ABI, headless
cd web && npm run check-audio
cd web && npm run check-lobby         # needs the server running
cd web && npm run check-match         # needs the server running
curl http://localhost:8080/health     # -> ok
```

`check-lobby` and `check-match` drive the real protocol over a socket, so start
`cargo run` first. Point either at a tunnel with `LOBBY_URL=wss://…/ws`.

Anything visual — the pixel face, wrapping, focus rings, the lobby's dropdown —
is checked in a real browser through the Playwright MCP server (see the trap in
`handoff.md`): build, `npx next start -p 3009`, then navigate, evaluate and
screenshot. Screenshots land in `.playwright-mcp/`, which is gitignored.

## Sharing with cloudflared

Both routes are in `docs/sharing.md`. Summary:

**Quick tunnel** — no account, random link, dies on Ctrl-C.

```sh
# single player only: the game, then the tunnel
cd web && npm run build && npm start
cd web && npm run share               # prints https://<words>.trycloudflare.com
```

**Quick tunnel including the lobby** — a quick tunnel forwards to one port, so
the socket needs its own:

```sh
cd server && cargo run                # terminal 1
cd web && npm start                   # terminal 2
cd web && npm run share               # terminal 3 -> the game link
cloudflared tunnel --url http://localhost:8080   # terminal 4 -> the socket link
```

Then join them and send the result:

```
<game link>/lobby?server=wss://<socket link>/ws
```

**Named tunnel** — a Cloudflare account **and a domain you own** (the free tier
includes the tunnel, not the name), but one stable link and no `?server=` at all,
because both are on one hostname. Two ways to set it up, both in
`docs/sharing.md`: from the dashboard (**Networking → Tunnels → Add route →
Published application**; add `/ws` → `http://localhost:8080` *first*, then the
bare hostname → `http://localhost:3000`), or from `deploy/cloudflared.yml` with
the `cloudflared tunnel …` commands:

```sh
cloudflared tunnel login
cloudflared tunnel create tank-in-city
cloudflared tunnel route dns tank-in-city tank.example.com
cloudflared tunnel --config deploy/cloudflared.yml run   # after filling it in
```

→ `https://tank.example.com/lobby`

Rules that matter: `npm start` and never `npm run dev` for strangers; the link is
the only secret there is; stop the tunnel when you are done.

## Troubleshooting

| Symptom | Cause |
|---|---|
| Friend gets a 502 | the tunnel is up, the thing behind it is not — wrong port, or not started |
| Lobby: "could not reach the game server" | the socket tunnel is missing or `?server=` is wrong; the page shows what it tried |
| `npm run share` refuses to start | it checks the port first, on purpose |
| `Cannot determine default configuration path` | normal for a quick tunnel, ignore |
| `ICMP proxy feature is disabled` | normal on Linux, ignore |
| Page loads but looks wrong | hard-refresh; the browser cached the old link |
| `localhost:8080/health` fails | the server is not running — `cd server && cargo run` |

## Files

| File | What it is |
|---|---|
| `web/scripts/share.sh` | quick tunnel, checks the port first |
| `web/scripts/check-lobby.mjs`, `check-match.mjs` | drive the protocol against a running server |
| `web/scripts/build-wasm.sh` | `game-core` to `web/public/game_core.wasm` |
| `deploy/cloudflared.yml` | the named tunnel |
| `docs/sharing.md` | the long version: tunnels, security, what was verified |
