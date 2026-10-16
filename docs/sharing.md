# Sharing the game with friends

How to put Tank In City on a public link so other people can open it, without
deploying anything. It uses a **Cloudflare Tunnel** (`cloudflared`), which
forwards a public `https://` address to something running on your machine.

Two ways:

| | Needs | Link | Use it for |
|---|---|---|---|
| **Quick tunnel** | nothing | random, new each run | testing with friends today |
| **Named tunnel** | a Cloudflare account and a domain | stable, yours | a link you can keep |

Start with the quick tunnel. It is one command and it needs no account.

## What friends can do right now

**Single Player, and the lobby. Not a match.**

- **Single Player** works fully. That is one link and nothing else.
- **The lobby** works: friends can create a room, join by code, appear on a side,
  mark ready, and the host can start. You will see each other.
- **A match does not.** Pressing start moves the room to "playing" and stops
  there: the match loop is not built. The engine is still single player, so it
  cannot hold two tanks yet.

So invite someone to prove the socket works, not to play a game against you.

## Quick tunnel

### Single Player only

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

`cloudflared` prints a box with a link ending in `trycloudflare.com`. That is the
link to send. It stays alive until you press Ctrl-C.

### To include the lobby

Four terminals. The socket needs its own tunnel, because a quick tunnel can only
forward to one port.

```sh
# Terminal 1: the game server (listens on 127.0.0.1:8080)
cd server
cargo run

# Terminal 2: the game itself
cd web
npm start

# Terminal 3: a tunnel to the game
cd web
npm run share

# Terminal 4: a tunnel to the socket
cloudflared tunnel --url http://localhost:8080
```

Now take the two links cloudflared printed and join them:

```
<terminal 3 link>/lobby?server=wss://<terminal 4 link>/ws
```

For example, if terminal 3 gave you `https://quiet-hill-abc.trycloudflare.com`
and terminal 4 gave you `https://fast-river-xyz.trycloudflare.com`, the link to
send is:

```
https://quiet-hill-abc.trycloudflare.com/lobby?server=wss://fast-river-xyz.trycloudflare.com/ws
```

That is the whole trick. `?server=` exists because the address changes every run,
so it cannot be baked into the build; see "When the match loop lands" below.

The lobby page shows the address it is using under the title, so a wrong one is
visible rather than mysterious.

## Is the socket working?

Before inviting anyone, check the server on its own:

```sh
cd web && npm run check-lobby
```

It connects twice, creates a room, joins it, readies both players, starts the
match, and leaves. Five seconds, and it tells you whether the problem is the
server or the browser.

Point it at a tunnel with `LOBBY_URL`:

```sh
LOBBY_URL=wss://fast-river-xyz.trycloudflare.com/ws npm run check-lobby
```

`curl http://localhost:8080/health` answering `ok` means the server is up.

## What was verified

Checked end to end against real tunnels, not assumed:

| What | Result |
|---|---|
| The public link served the page, the wasm, the audio and the font | all 200 |
| `next dev` behind the same tunnel | every route 200 |
| A WebSocket round-tripped through a quick tunnel | reply received |
| **The whole lobby protocol over a quick tunnel, `wss://`** | create, join, ready, start, leave |
| The game server's `/health` and `npm run check-lobby` | 200, and `lobby ok` |

**Not verified:** anything needing a browser. Nobody has opened `/lobby` in one,
so no button has been clicked and no audio has been heard. Only the HTTP and
WebSocket traffic was checked.

## Security, before you leave it running

A quick tunnel puts your machine on the public internet at an unlisted address.
Anyone with the link gets through.

- **Use `npm start`, not `npm run dev`.** The dev server serves source maps and
  is not built for strangers.
- **Stop it when you are done.** Ctrl-C closes a tunnel. It does not keep
  running on its own.
- **Do not leave it up overnight.** It is a testing tool, not hosting.
- **The link is the only secret.** There is no password on it.
- **The game server binds `127.0.0.1` only**, so the tunnel is the only way in
  and nothing else on your machine is reachable.
- **A room password is not security.** It is a lobby gate and it travels in the
  clear. See `docs/deferred.md`.

## Named tunnel, for a stable link

Do this when the random link each run gets annoying. It needs a Cloudflare
account and a domain on that account. Everything is in `deploy/cloudflared.yml`.

```sh
cloudflared tunnel login
cloudflared tunnel create tank-in-city
cloudflared tunnel route dns tank-in-city tank.example.com
```

Then fill the UUID, the credentials path and the hostname into
`deploy/cloudflared.yml` and run it:

```sh
cloudflared tunnel --config deploy/cloudflared.yml run
```

The config sends `/ws` to the game server on 8080 and everything else to the game
on 3000, **on one hostname**. That is deliberate: the lobby then opens
`wss://<its own host>/ws`, so there is no `?server=` to add and no second link to
send. It is also the only arrangement the browser allows — a page served over
`https` cannot open a plain `ws://` socket at all, it blocks it as mixed content.

So with a named tunnel the link is just:

```
https://tank.example.com/lobby
```

## When the match loop lands

Two things are already handled, and one is left:

1. **The socket's route** — done, above.
2. **A WebSocket through the tunnel** — verified working.
3. **Telling the client where the socket is.** Today that is `?server=`. Once
   the lobby is reachable on the same hostname as the game (a named tunnel, or a
   deployment that serves both), the default is already right and the parameter
   is only needed for quick tunnels.

## Troubleshooting

Messages seen while testing this, and what they mean:

- **`Cannot determine default configuration path`** — normal. No named tunnel
  config was found, which is right for a quick tunnel. Ignore it.
- **`ICMP proxy feature is disabled` / `ping_group_range`** — normal on Linux
  desktop. cloudflared wanted to ping through the tunnel and was not allowed to.
  Nothing you need uses that.
- **`Environment is healthy`** — the pre-flight check passed. Good.
- **A friend gets a 502** — the tunnel is up but the thing behind it is not.
  Check the right server is running on the right port.
- **The lobby says "could not reach the game server"** — the socket tunnel is
  missing, or `?server=` points at the wrong one. The page shows the address it
  tried. `npm run check-lobby` will tell you whether the server is fine.
- **`npm run share` refuses to start** — it checks the port first, on purpose. A
  tunnel to a dead port gives friends a 502 with no explanation.
- **The page loads but looks wrong** — hard-refresh. The browser may hold a
  cached copy from a previous link.
- **Edits do not show up with `npm run dev`** — hot reload over a tunnel is not
  reliable. Reload the page. Friends do not need it.
- **cloudflared metrics** — it opens `127.0.0.1:20241/metrics` on your machine,
  localhost only, for its own health numbers. Nothing to do with the game.

## Files

| File | What it is |
|---|---|
| `web/scripts/share.sh` | the quick tunnel, checks the port first, mentions the socket tunnel |
| `web/scripts/check-lobby.mjs` | drives the lobby protocol against a running server |
| `deploy/cloudflared.yml` | the named tunnel: `/ws` to the server, the rest to the game |
| `docs/sharing.md` | this file |
