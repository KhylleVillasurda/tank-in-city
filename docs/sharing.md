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
- **A match does not render.** The server runs it — one `World` per room at 60 Hz,
  pushing `map` and `snapshot` — but no match page exists in the browser yet, so
  pressing start leaves the room in "playing" with nothing drawing it.

So invite someone to prove the socket works, not to play a game against you.
Building and running the pieces is in `docs/running.md`.

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

**Wait about thirty seconds before anyone opens it.** `cloudflared` prints the
address before Cloudflare has published a DNS record for it, and a lookup that
arrives inside that gap gets `NXDOMAIN` — which your resolver then remembers for
the length of its negative cache. So the link can look dead to *you* for minutes
while it is already fine for everyone else. Do not try it in your own browser
first. If it is already poisoned, check with a public resolver, which has no such
memory: `dig @1.1.1.1 <the-link-host>` answers while your own resolver may not.

### No tunnel at all, for playing against yourself

The lobby only reaches for a tunnel when the page is on `https`. Served from
`localhost` over plain `http`, `serverUrl()` returns `ws://localhost:8080/ws`, so
with `cd server && cargo run` in another terminal:

```
http://localhost:3000/lobby
```

is a complete room — two browser tabs, two guests, ready, start. No tunnel, no
domain, no DNS. This is the fastest way to exercise the lobby and, once the match
page exists, a real 1v1.

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
- **Stop it when you are done.** Ctrl-C closes a quick tunnel and the one started
  with `--config`. A dashboard tunnel is a **service** and keeps running after you
  close the terminal, and after a reboot: `sudo systemctl stop cloudflared`, and
  `sudo systemctl disable cloudflared` if it should not come back on its own.
- **Do not leave it up overnight.** It is a testing tool, not hosting.
- **The link is the only secret.** There is no password on it.
- **The game server binds `127.0.0.1` only**, so the tunnel is the only way in
  and nothing else on your machine is reachable.
- **A room password is not security.** It is a lobby gate and it travels in the
  clear. See `docs/deferred.md`.

## Named tunnel, for a stable link

Do this when the random link each run gets annoying. It needs a Cloudflare
account **and a domain you own** — the free tier includes the tunnel, not the
name. A published application's hostname is `<subdomain>.<your-domain>`, so with
no domain registered there is nothing to put after the dot and the named tunnel
cannot serve anything; `*.trycloudflare.com` random addresses are the free
hostname, and they come from the quick tunnel above. Cloudflare's other free
hostnames (`*.pages.dev`, `*.workers.dev`) are Cloudflare's own and cannot be
pointed at a tunnel.

It is also the arrangement the browser insists on anyway: the game page is
`https`, and a page served over `https` is not allowed to open a plain `ws://`
socket, so the socket has to arrive over `wss://` too.

### From the dashboard (remotely managed)

This is the one to use when the tunnel shows as **Healthy** under **Networking →
Tunnels** and `cloudflared.service` is running: the routes live in Cloudflare and
the token sits inside the systemd unit, so `~/.cloudflared/` stays empty and
`deploy/cloudflared.yml` is **not used at all**. Nothing to start by hand — the
service reconnects on its own after a reboot.

Add the routes under the tunnel's **Routes** tab, **Add route → Published
application**. Two of them, and the order matters:

| Subdomain | Path | Service URL | Why |
|---|---|---|---|
| `tank` | `/ws` | `http://localhost:8080` | the game server's socket |
| `tank` | *(empty)* | `http://localhost:3000` | the game itself |

**Add `/ws` first.** Rules are tried in the order they are listed and the first
match wins, so a hostname-wide route added first swallows `/ws` and the socket
gets Next.js's own 404. The symptom is the lobby saying "could not reach the game
server". If both routes exist in the wrong order, delete both and add them again
in this order.

**The hostname has to carry the domain.** The form builds it from the
**Subdomain** field and the **Domain** dropdown, so the Destination has to read
something like `tank-in-city.example.com`; a bare `tank-in-city` is not a public
hostname and nothing resolves it, even though the dashboard saves it happily and
`cloudflared` reports a healthy tunnel. An empty Domain dropdown means the domain
is not a zone on this Cloudflare account yet — add the website to Cloudflare
first. Prefer hyphens to underscores: `_` is not valid in a hostname.

Then the link is the whole thing:

```
https://tank.example.com/lobby
```

No `?server=` — with the page on `https`, `serverUrl()` in `web/game/lobby.ts`
opens `wss://<its own host>/ws`, which is exactly where the `/ws` route points.

Check it worked, with both local servers up (`cd web && npm start`, `cd server &&
cargo run`):

```sh
curl https://tank.example.com/health                    # -> ok, from the game server
LOBBY_URL=wss://tank.example.com/ws npm run check-lobby # -> lobby ok
```

`journalctl -u cloudflared | grep "Updated to new configuration"` prints the
ingress rules Cloudflare pushed down, in order, which is the fastest way to see
whether the dashboard kept them the way you meant.

### Two hostnames instead

If the dashboard will not keep `/ws` in front of the hostname-wide route, give
the socket a name of its own. Order cannot bite, because the two never compete:

| Hostname | Service URL |
|---|---|
| `tank.example.com` | `http://localhost:3000` |
| `tank-ws.example.com` | `http://localhost:8080` |

```
https://tank.example.com/lobby?server=wss://tank-ws.example.com/ws
```

Both hostnames still need their own DNS route, which **Add route → Published
application** creates for you.

### From the command line (a local config file)

The original arrangement, for when there is no dashboard tunnel to add to. It
keeps the routes in `deploy/cloudflared.yml`, which is checked in, so the order
is visible in the repository:

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

Same two rules, same order — `/ws` to 8080 first, then everything else to 3000 —
but they are lines in that file rather than entries in a list. One difference:
the UUID and the credentials file have to be filled in, and this tunnel only
runs while that command does.

## Telling the client where the socket is

`?server=` exists only because a *quick* tunnel hands out a new address every
run and puts the game and the socket on two different hosts; nothing can be baked
into a build that changes each time. With a named tunnel on one hostname the
default is already right, and the parameter is only needed for the two-hostname
arrangement above.

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
  missing, or `?server=` points at the wrong one, or (on a named tunnel where
  both are on one hostname) the `/ws` route is not first, so the hostname-wide
  route swallowed it. The page shows the address it tried. `npm run check-lobby`
  will tell you whether the server is fine.
- **`cloudflared.service` is active but the hostname gives a 502** — the tunnel
  is connected and nothing is behind the route. Check the service URL in the
  dashboard (`http://localhost:3000`, `http://localhost:8080`) and that both
  local servers are actually running.
- **A hostname that resolves but never answers** — the DNS route was never
  created. **Add route → Published application** creates it; adding only a
  CNAME by hand does not.
- **The tunnel is healthy and the routes look right, but the hostname does not
  resolve at all** — the route's Destination has no domain on it. `journalctl -u
  cloudflared | grep "Updated to new configuration"` shows exactly what Cloudflare
  pushed down: `{"hostname":"tank-in-city", ...}` is a bare label and cannot
  resolve, where `{"hostname":"tank-in-city.example.com", ...}` can.
- **`npm run share` refuses to start** — it checks the port first, on purpose. A
  tunnel to a dead port gives friends a 502 with no explanation. The message
  names the fix: the game has to be running, `cd web && npm start` in another
  terminal. This is the script working, not failing.
- **The link works for your friend but is dead for you** — you looked it up
  before Cloudflare had published the record and your resolver cached the
  `NXDOMAIN`; see "Wait about thirty seconds" above. `dig @1.1.1.1 <host>` proves
  whether the name exists. Itself harmless: the record is fine, your local cache
  is not.
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
| `deploy/cloudflared.yml` | the named tunnel *from the command line*: `/ws` to the server, the rest to the game. Unused when the tunnel is managed in the dashboard |
| `docs/running.md` | build, run and check commands, and the tunnel commands in brief |
| `docs/sharing.md` | this file |
