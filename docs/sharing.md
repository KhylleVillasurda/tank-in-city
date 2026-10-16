# Sharing the game with friends

How to put Tank In City on a public link so other people can open it, without
deploying anything. It uses a **Cloudflare Tunnel** (`cloudflared`), which
forwards a public `https://` address to a server running on your machine.

Two ways, in `deploy/cloudflared.yml` and `web/scripts/share.sh`:

| | Needs | Link | Use it for |
|---|---|---|---|
| **Quick tunnel** | nothing | random, new each run | testing with friends today |
| **Named tunnel** | a Cloudflare account and a domain | stable, yours | a link you can keep |

Start with the quick tunnel. It is one command and it needs no account.

## What friends can do right now

**Single Player. That is all.** Read this before inviting anyone.

Multiplayer is not built yet. The engine is single player, the server has no
socket, and the lobby has no client. So two friends opening the link will each
get their own Single Player run and will not see each other.

That is Milestone 3 slices 2 and 3. The tunnel below is the transport they will
use, so setting it up now is not wasted — but do not promise a friend a match.

## Quick tunnel

Two terminals.

```sh
# Terminal 1: build once, then serve
cd web
npm run build
npm start
```

```sh
# Terminal 2: open the tunnel
cd web
npm run share          # or: bash scripts/share.sh
```

`cloudflared` prints a box with a link ending in `trycloudflare.com`. That is
the link to send. It stays alive until you press Ctrl-C.

`npm run dev` works too, and is fine for a quick look, but it is slower and the
dev server recompiles as you type. `npm start` is the production build and is
what you want when someone else is playing.

`PORT=4000 npm run share` points the tunnel at a different port, if your server
is not on 3000.

## What was verified

This was tested end to end, not assumed. A production build behind a quick
tunnel, fetched from the public link:

| Request | Result |
|---|---|
| `/`, `/menu`, `/game` | 200 |
| `/game_core.wasm` | 200, 41,360 bytes |
| `/audio/shoot.ogg` | 200, 5,408 bytes |
| the pixel font | 200, 12,512 bytes |
| `next dev` behind the same tunnel | all three routes 200 |

**A WebSocket survives the tunnel.** That was tested separately with a throwaway
echo server on port 8080 behind a second quick tunnel: the message went out and
the reply came back. This matters because it is the transport multiplayer will
use, and it means multiplayer over a quick tunnel needs no account and no domain.

**Not verified:** anything needing a browser. Nobody has clicked the link from a
real browser, and no audio has been heard. Only the HTTP and WebSocket traffic
was checked.

## Security, before you leave it running

A quick tunnel puts your machine on the public internet at a guessable-looking
but unlisted address. Anyone with the link gets through.

- **Use `npm start`, not `npm run dev`.** The dev server serves source maps and
  is not built for strangers. The production server is.
- **Stop it when you are done.** Ctrl-C closes the tunnel. It does not keep
  running on its own.
- **Do not leave it up overnight.** It is a testing tool, not hosting.
- **The link is the only secret.** There is no password on it. Share it where
  you would share anything else public.
- The tunnel exposes **only the port you point it at**. Nothing else on your
  machine is reachable.

## Named tunnel, for a stable link

Do this when the random link each run gets annoying, or when you want both the
game and the game server on one tunnel. It needs a Cloudflare account and a
domain added to that account. Everything is in `deploy/cloudflared.yml`.

```sh
cloudflared tunnel login
cloudflared tunnel create tank-in-city
cloudflared tunnel route dns tank-in-city tank.example.com
cloudflared tunnel route dns tank-in-city tank-ws.example.com
```

Then fill the UUID and paths into `deploy/cloudflared.yml` and run it:

```sh
cloudflared tunnel --config deploy/cloudflared.yml run
```

The file already has an ingress rule for the game server on port 8080. Nothing
listens there yet — it is slice 2 — so that rule starts working the day the
socket lands.

## When multiplayer arrives

Two things change, and both are already handled by the config above.

1. **The socket needs its own hostname.** A WebSocket upgrade wants its own
   route, and the game page is served over `https`. A plain `ws://` connection
   from an `https` page is blocked by the browser as mixed content, so the
   socket has to arrive as `wss://` through a tunnel as well. That is the
   `tank-ws.example.com` rule.
2. **The client needs to be told where the socket is.** With a quick tunnel the
   address changes every run, so it cannot be hard-coded. Expect either a
   `NEXT_PUBLIC_` environment variable or a query parameter on the link. Note
   that Next bakes `NEXT_PUBLIC_*` in at build time, so changing it means
   rebuilding — the query parameter avoids that.

## Troubleshooting

Messages seen while testing this, and what they mean:

- **`Cannot determine default configuration path`** — normal. It means no named
  tunnel config was found, which is exactly right for a quick tunnel. Ignore it.
- **`ICMP proxy feature is disabled` / `ping_group_range`** — normal on Linux
  desktop. cloudflared wanted to ping through the tunnel and was not allowed to.
  Nothing you need uses that.
- **`Environment is healthy`** — the pre-flight check passed. Good.
- **A friend gets a 502** — the tunnel is up but your server is not. Check that
  `npm start` is still running and that it is on the same port the tunnel points
  at.
- **`npm run share` refuses to start** — it checks the port first, on purpose. A
  tunnel to a dead port gives friends a 502 with no explanation, which is worse
  than a clear error here.
- **The page loads but looks wrong** — hard-refresh. The browser may hold a
  cached copy from a previous link.
- **Editing does not show up with `npm run dev`** — hot reload over a tunnel is
  not reliable. Reload the page. Friends do not need it, so this only affects
  you.
- **cloudflared metrics** — it opens `127.0.0.1:20241/metrics` on your machine,
  localhost only, for its own health numbers. Nothing to do with the game.

## Files

| File | What it is |
|---|---|
| `web/scripts/share.sh` | the quick tunnel, checks the port first, prints the caveats |
| `deploy/cloudflared.yml` | the named tunnel, both hostnames, ready to fill in |
| `docs/sharing.md` | this file |
