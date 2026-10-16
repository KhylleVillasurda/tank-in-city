#!/usr/bin/env bash
#
# Share the game with friends over a Cloudflare quick tunnel.
#
# A quick tunnel needs no account, no domain and no config file. cloudflared
# hands you a random https://<words>.trycloudflare.com link that forwards to
# your machine, and it lives only as long as this command runs.
#
#   cd web && npm run build && npm start     # one terminal
#   bash scripts/share.sh                    # another
#
# What friends can actually do is in docs/sharing.md. Short version: they can
# play Single Player. Multiplayer is not built yet, so they cannot play each
# other. This tunnel is the transport it will use when it is.

set -euo pipefail

PORT="${PORT:-3000}"
TARGET="http://localhost:${PORT}"

if ! command -v cloudflared >/dev/null; then
  cat >&2 <<'EOF'
cloudflared is not installed.

  https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/

On Debian or Ubuntu, the .deb from that page installs it in one step.
EOF
  exit 1
fi

# A tunnel to a dead port gives friends a 502 and no clue why, so check first.
if command -v curl >/dev/null; then
  if ! curl -sf -o /dev/null --max-time 5 "$TARGET"; then
    cat >&2 <<EOF
Nothing is answering on $TARGET.

Start the game first, in another terminal:

  cd web && npm run build && npm start

npm start is the production server and is the one to share. npm run dev works
too, but it is slower and the link is only as stable as the dev server.
EOF
    exit 1
  fi
else
  echo "curl is not installed, skipping the reachability check." >&2
fi

cat <<EOF

Tunnelling $TARGET to a public https:// link.

  cloudflared prints the link in a box below. It is the one ending in
  trycloudflare.com. Send that to your friends. It stops when you press Ctrl-C.

  Anyone with the link can reach this machine while it runs, so stop it when
  you are finished.

  Friends can play Single Player. They cannot play each other yet: multiplayer
  is not built. See docs/sharing.md.

EOF

# exec so Ctrl-C reaches cloudflared directly and the tunnel closes with it.
exec cloudflared tunnel --url "$TARGET"
