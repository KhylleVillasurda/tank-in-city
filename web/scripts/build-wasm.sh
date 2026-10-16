#!/usr/bin/env bash
# Build game-core for the browser and drop the module where Next.js serves it.
# Plain cargo + wasm32: no wasm-pack, no wasm-bindgen. See docs/day-1.md.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(cd "$here/../.." && pwd)"

cargo build --manifest-path "$root/game-core/Cargo.toml" \
  --target wasm32-unknown-unknown --release

mkdir -p "$here/../public"
cp "$root/game-core/target/wasm32-unknown-unknown/release/game_core.wasm" \
  "$here/../public/game_core.wasm"

echo "web/public/game_core.wasm updated"
