# Tank In City: Project Plan

## 1. Overview

A browser-based, top-down tank game inspired by the mechanics of Battle City (1990). Players play solo against AI, or host or join a multiplayer room, as a guest or a signed-in account. Accounts track match history and leaderboard standing.

**Working title:** Tank In City. Do not use "Battle City" or its logo. Mechanics are fine to reuse; branding is not.

**Assets:** Original pixel art, or CC0 or free-licensed material with credits (see Section 12). Do not use sprites or sounds ripped from the original game.

**Language:** English only for now.

**Platform:** Desktop browsers only for now. No mobile controls.

## 2. Accounts

- **Sign up:** Username and password, plus an email address for verification.
- **Email verification:** Gates saved match history and the leaderboards only. An unverified account can play, host, and join exactly like a guest; its matches simply aren't saved until it verifies.
- **Password reset:** Via emailed link.
- **Passwords:** Hashed with Argon2. Never stored or logged in plain text.
- **Login:** Session stored in an HttpOnly, Secure, SameSite cookie. Rate-limit failed logins.
- **Session expiry:** 30 days, refreshed on each use.
- **Account deletion:** A signed-in user can delete their account. This removes their credentials, sessions, and email tokens. Their finished-match rows are anonymized (name and score kept, `user_id` set to null) so other players' match history stays intact.
- **Privacy:** Store only what's needed (username, email, password hash). Publish a short privacy page covering what's kept and how the user deletes it.
- **Usernames:** Unique, 3–12 characters, letters, numbers, and underscores only. The username is the in-game name.
- **Guest play:** Guests can play Single Player, host, and join multiplayer rooms. They are given a generated name (for example `SilentTank42`), which must be unique within a room. Their matches are not saved to history and they have no total score.

## 3. Player Flow

1. **Title screen:** Log in, create an account, or play as a guest.
2. **Main menu:**
   - **Single Player:** Endless arcade run against AI tanks. Runs entirely in the browser.
   - **Host Room:** Pick a mode and options, then get a room code.
   - **Join Room:** Browse public rooms, or enter a room code (and password, if set).
   - **Profile:** Total score, match history, leaderboard standing.
3. **Host options:**
   - **Mode:** 1v1, 2v2, 3v3, Free For All (2–6 players), or Co-op vs AI (exactly 2 players)
   - **Friendly fire:** On or off (team modes only; FFA is always on)
   - **Public or private:** Public rooms appear in the room list. Private rooms are joined by code only.
   - **Room password:** Optional
4. **Lobby:** Players join, see teams, and mark ready. The host starts the match.
5. **Match:** Results screen shows scores, then return to lobby or menu.

### Room list
- Public rooms show: room code, host name, mode, player count, and status (waiting or in progress).
- Rooms in progress stay listed but can't be joined.
- The list refreshes automatically.

### Room modes
| Mode | Players | Teams | AI tanks | Friendly fire | Win condition |
|---|---|---|---|---|---|
| 1v1 | 2 | Yes | No | Host sets | Enemy base destroyed, or all enemy players out of lives. Whichever happens first wins. |
| 2v2 | 4 | Yes | No | Host sets | Same as 1v1 |
| 3v3 | 6 | Yes | No | Host sets | Same as 1v1 |
| Free For All | 2–6 | No | No | Always on | Last player with lives, or highest score at time limit (tie goes to sudden death) |
| Co-op vs AI | 2 | Same side | Yes | Off | Survive all waves, or base falls |

- Co-op vs AI is the only multiplayer mode with AI tanks. It's limited to 2 human players.
- Team modes never have AI tanks.

### Room rules
- Room codes are 5 characters. The room closes when empty.
- **No joining after the match starts.** The room shows "in progress" and rejects new joins.
- **Host leaves:** Host passes to the next player who joined. If the room is empty, it closes.
- **Names:** Each player's name is their username, or a generated guest name. Two players with the same name can't be in one room.

### Teams in the lobby
- Teams show as players join. A new player goes to the smaller side (ties go to team A).
- Players can request a swap with someone on the other team. The other player accepts or declines.
- The host can also move players.
- **Uneven teams are allowed.** The host can start with an uneven count, such as 3 vs 2. Counts show in the lobby before the start.
- **Start conditions:** a match starts only when every player is ready. 1v1 starts with 1 per side; every other team mode needs at least 2 per side, and may start uneven (for example 2v3). No side may exceed the mode's player count.

### Friendly fire
- Set by the host, shown to everyone before the match starts, and locked once it begins.
- **Off:** Bullets pass through teammates without damage.
- **On:** Teammates can damage each other. Bullets still stop at walls.

## 4. Engine Architecture (WebAssembly)

The game rules live in one Rust crate, `game-core`. It has no network, DOM, or database code. It compiles two ways:

- **To WebAssembly** for the browser. Used for Single Player (fully local) and for client-side prediction of the player's own tank.
- **To native code** for the server. The server runs the same rules to decide hits, scores, and spawns.

### Simulation rules
- **Fixed timestep:** 60 steps per second, independent of frame rate.
- **Integer math:** Positions, velocities, and timers are fixed-point integers, not floats. Float math is not bit-identical across WASM and native CPUs, and that divergence would desync client prediction from the server.
- **Seeded RNG:** Enemy spawns and AI decisions come from a seeded random number generator. Same seed plus same inputs gives the same result on every platform (WASM and native).
- **Input replay:** Any match can be replayed from its seed and input log. This is used for tests and bug reports.

### Tank collisions
- Tanks are solid. Two tanks can't overlap; a tank that drives into another stops at the edge.
- This applies to all tanks: player-to-player, player-to-enemy, and enemy-to-enemy.
- Tanks can't drive through walls, water, or the base.

### Who does what
- **Client (browser):** Renders the canvas, reads input, predicts the local player's tank, and interpolates other tanks between snapshots.
- **Server:** Owns the authoritative state for each multiplayer room.
- **Single Player:** The browser runs the engine alone. No server connection.

## 5. Networking

- **Transport:** WebSocket, JSON messages, through Axum.
- **Map transfer:** The server sends the full tile map on join and at the start of each round. Later changes (brick destroyed, steel placed) arrive as `map_event` deltas.
- **Snapshots:** The server sends a full state snapshot at 15 Hz. `tanks` covers both players and AI.
- **Input:** The client sends input with a tick number, at most once per simulation step.
- **Client-side prediction:** The local player's tank moves immediately using the same WASM engine. When a snapshot arrives, the client snaps its own tank to the server position if the gap is larger than a small threshold, and keeps predicting otherwise.
- **Missing input:** If no input arrives for a tick, the server repeats the player's last input, so a held key keeps moving.
- **Interpolation:** Other tanks are drawn between their last two snapshot positions.
- **Session tokens:** When a player joins, the server issues a random session token, kept in sessionStorage. Reconnecting requires this token.

## 6. Rules

Numbers are starting defaults to tune.

### PvP lives and respawn
- Each player starts with **3 lives** in team modes and FFA.
- A destroyed tank respawns next to its own base after **3 seconds**, with **2 seconds** of invulnerability.
- A tank with respawn or reconnect invulnerability **cannot fire** (defense only).
- Losing all lives eliminates the player. They become a spectator.
- In FFA, at the time limit the highest score wins; a tie goes to sudden death (next kill or elimination decides it).
- A match stops early if it becomes unplayable: when only one player remains, or, in team modes, when a whole side is empty.

### PvP match structure
- A PvP match is a **sequence of maps**, not a single map. When a round ends, the next map loads.
- A round ends when one team's base is destroyed or all of a team's players are out of lives. FFA rounds end at the time limit, or when one player remains.
- Each new round resets lives, scores, power-ups, and tank upgrades. Only the round count carries over.
- **Default: best-of-3** (first to win 2 rounds takes the match). Tune later.
- The match cycles through different maps; PvP ships with at least 5 maps (see Milestone 5).

### Spectating
- Eliminated players can watch the rest of the match, switching between remaining players' views.
- Spectators can't join the match, chat, or affect it.
- **Decision (subject to change): watch only.** Alternatives, if this is ever revisited: let spectators re-queue for the next match while watching, or add chat (which the game doesn't have at all). Either change is isolated to the client's spectator mode and the lobby flow.

### Bullets
- A tank may have only **1 bullet** in flight at a time. A fully upgraded player tank may have **2**.

### Scoring (default)
- Enemy tank kill (Single Player and Co-op): Basic 100, Fast 200, Power 300, Armor 400.
- PvP kill: 500 points.
- Base destroyed (team modes): 1000 bonus points to the player who destroys the base. It counts toward that player's contribution and the team's total.
- **Team-kill penalty (friendly fire on):** -250 points, and the victim counts a death. Team kills do not count as kills.
- **Contribution vs total:** the number saved to an account is the player's own contribution (kills plus bonuses). The team's combined total is shown in-game only and is never stored.
- Match contributions are added to the account's running total. Guest scores are not saved.
- Disconnected or quitting players keep the contribution they had when they left.

### Enemy waves (Single Player and Co-op)
- 20 enemies per stage by default, spawning from three points at the top of the map.
- A spawn point flashes briefly before a tank appears.
- Up to 4 enemies on the map at once.
- Spawn order is random, using the seeded RNG.

### Enemy types
- **Basic:** Slow, 1 hit.
- **Fast:** Quick, 1 hit.
- **Power:** Fast bullets, 1 hit.
- **Armor:** Takes several hits. 4 by default, rising in later stages, **maximum 10**. Its color changes with each hit so damage is visible.

### Enemy behavior
- Moves in straight lines, changes direction at intervals or when blocked.
- Fires at intervals. Sometimes heads toward the base.
- Some enemies carry a power-up that drops when destroyed.

### Base
- Surrounded by brick walls. Walls can be upgraded to steel as a power-up.
- **Each team defends exactly one base.** A team loses when its base is destroyed.
- In Single Player and Co-op, losing the base ends the stage or match.
- In team modes, the base is a win condition. FFA has no base.

### Power-ups
| Power-up | Single Player | Co-op vs AI | Team modes | FFA |
|---|---|---|---|---|
| Extra life | Yes | Yes | Yes | Yes |
| Temporary shield | Yes | Yes | Yes | Yes |
| Freeze enemies | Yes | Yes | **No** | **No** |
| Destroy all enemies on screen | Yes | Yes | Opposing team only | All other players |
| Tank upgrade (faster bullets, then two-shot) | Yes | Yes | Yes | Yes |
| Speed boost (movement speed) | Yes | Yes | Yes | Yes |
| Steel base walls | Yes | Yes | Yes | No (no base in FFA) |

Power-up durations (defaults to tune):
- Shield: 8 seconds.
- Freeze enemies: 8 seconds.
- Speed boost: 10 seconds.
- Tank upgrade: lasts until the tank is destroyed, then resets.
- Steel base walls: 15 seconds, then reverts to brick.
- Extra life and destroy-all-enemies apply immediately.

### Power-up spawns
- A power-up drops when a bonus (flashing) enemy is destroyed. Default: the 4th, 11th, and 18th enemies of a stage carry one.
- At most one power-up lies on the map at a time; a new drop replaces an uncollected one.
- Power-ups drop at the destroyed enemy's tile, anywhere on the map.
- Enemies that drive over a power-up collect it and gain its effect, denying it to the players.

### Power-ups an enemy collects
An enemy that takes a power-up applies it to **its own side**, not the player's:
- **Extra life:** survives one extra hit (functions as +1 hit point).
- **Temporary shield:** invulnerable for the shield duration (8 seconds).
- **Freeze enemies:** freezes the opposing side (the human players) for the same 8 seconds.
- **Destroy all enemies on screen:** destroys every opposing tank on the map (all human players). A deliberate big swing.
- **Tank upgrade:** faster bullets, then a second simultaneous shot.
- **Speed boost:** moves faster (10 seconds).
- **Steel base walls:** reinforces its own team's base for 15 seconds, the same as the player's version. In Single Player and Co-op the enemy has no base, so it simply consumes the pickup and robs the player of it.

### Single Player (endless arcade)
- Endless run: there is no final stage. Play until your lives run out; the result is the score and the highest stage reached.
- Starts with a set number of lives. Extra lives at fixed score thresholds.
- Clearing all enemies in a stage advances to the next stage, cycling through the stage maps.
- Difficulty rises with the stage number (more armor and fast enemies).

### Co-op vs AI
- Two human players on the same side, sharing one base.
- **Each player has their own lives.** The match ends when both are out of lives, or when the base falls.
- Eliminated players can spectate the other player.
- **Shared score:** the results screen shows one combined team total; each player's personal total still records only their own kills.
- Same wave system as Single Player.

### Disconnects and quitting
- The disconnected player's tank is removed right away, so enemies and bullets pass through where it stood. The server remembers its position and lives.
- Everyone sees "[Name] has disconnected."
- The player can reconnect within **30 seconds** with their session token, if the match is still running. They return at their remembered position, with the lives they had and **3 seconds** of invulnerability.
- After 30 seconds, the player is out. Their team plays on. In Co-op, the other player continues alone.
- **Quitting on purpose** (leaving the room during a match) eliminates the player the same way a final death would. Their contribution still counts if they are logged in.

## 7. Feature Milestones

Each milestone ends with a playable check, not just passing tests.

### Day 1: setup and first session

**Get these right from the very first commit:**
- **Integer/fixed-point math** for positions, velocities, and timers, plus the **fixed 60 Hz timestep**. This is the expensive thing to retrofit — start on floats and you will rewrite positions, collisions, and client prediction later.
- Keep `game-core` free of network, DOM, and database code from the start, so the same crate compiles to WASM and native.
- Nothing in the open questions blocks this work. Milestone 1 depends only on rules that are already decided.

**Suggested first-session order:**
1. Scaffold the `game-core` Rust crate.
2. Map tiles and the grid.
3. One player tank that moves in integer steps.
4. Bullets.
5. Collision (walls, bricks, tanks).
6. Compile the same crate to WASM.
7. Render the map and tank on an HTML5 Canvas.
8. Play it, then tune movement and shooting feel — this is the Milestone 1 check.

### Milestone 1: Vertical slice
One map, one player tank that moves and shoots, bricks that break, one enemy that moves and shoots, all on screen in the browser.
- [ ] `game-core` crate with map, tank, bullet, and collision logic
- [ ] Compiles to WASM and native
- [ ] Canvas renders the map and tanks
- [ ] Keyboard input (WASD or arrows, Space to fire), fixed timestep
- [ ] **Check:** Movement and shooting feel right after a few minutes of tuning

### Milestone 2: Single Player
- [ ] Full enemy waves and all four enemy types
- [ ] Base with brick walls, game over when it falls
- [ ] Lives, score, stage counter
- [ ] Endless run: score and highest stage reached, no final stage
- [ ] Power-ups (Single Player column)
- [ ] Stage data loading
- [ ] Pause, game over, next stage
- [ ] Title screen (log in, sign up, or play as guest)
- [ ] **Check:** A run can be played until you run out of lives, and a seeded replay reproduces it

### Milestone 3: Multiplayer
Rooms, modes, and the server game loop, built and tested together.
- [ ] Server runs `game-core` natively per room
- [ ] Basic structured logging (`tracing`): lifecycle and errors only, never per-tick state or secrets
- [ ] Create room (mode, friendly fire, public or private, optional password)
- [ ] Public room list; join by code
- [ ] Reject joins once a match has started
- [ ] Lobby with teams, swap requests, ready states, host controls
- [ ] Start conditions: all players ready, at least the mode's minimum per side
- [ ] Team modes (1v1, 2v2, 3v3), uneven starts allowed
- [ ] Free For All and Co-op vs AI
- [ ] Team win conditions (base or all lives, whichever comes first)
- [ ] PvP match as a best-of-3 sequence of maps; rounds reset lives, score, and upgrades
- [ ] Match stops early when unplayable (one player left, or a side empty)
- [ ] PvP lives, respawn, elimination, and spectating
- [ ] Solid tank collisions
- [ ] Client-side prediction and interpolation
- [ ] Session tokens, 30-second reconnect
- [ ] Host passes to next player when host leaves; empty rooms close
- [ ] **Check:** Two browser windows can play a full 1v1 with friendly fire on and off

### Milestone 4: Accounts and history
- [ ] PostgreSQL with SQLx migrations
- [ ] Sign-up with username, password, and email
- [ ] Email verification (gates history only) and password reset
- [ ] Login sessions in HttpOnly cookies, rate-limited logins
- [ ] Matches saved when they end (one transaction)
- [ ] Profile page: total score, match history, leaderboard standing
- [ ] Account deletion: anonymize match history, delete credentials, sessions, and email tokens
- [ ] **Check:** Create account, verify email, play a match, see it in history after logging out and back in

### Milestone 5: Content and polish
- [ ] Single Player: at least 5 stage maps for the endless run
- [ ] PvP: at least 5 maps, cycled between rounds
- [ ] One co-op map
- [ ] Balance pass on enemy counts, lives, and scoring
- [ ] Sound effects and music from CC0 or credited free sources
- [ ] Credits page listing every sound and asset source and license

## 8. Database

PostgreSQL, from Milestone 4 onward.

```
users         (id, username, email, password_hash, email_verified, total_vs_ai, total_multiplayer, created_at)
sessions      (id, user_id, token_hash, expires_at)
email_tokens  (id, user_id, token_hash, purpose, expires_at)   -- verification and reset
matches       (id, room_code, mode, category, friendly_fire, started_at, ended_at)  -- category: vs_ai | multiplayer
match_players (match_id, user_id, name, team, score, kills, deaths, placement, disconnected)
```

- The account's total is updated in the same transaction that saves a match.
- `match_players.score` is the player's own contribution. The team's combined total is not stored.
- Two leaderboards come from `users.total_vs_ai` (Single Player and Co-op) and `users.total_multiplayer`.
- `placement` is the player's score rank at the end of the match; tied players share a rank.
- Store token hashes, not raw tokens.
- Guest matches are not written to this database.
- Deleting an account deletes the user's `sessions` and `email_tokens`, and anonymizes their `match_players` rows (`user_id` set to null, name and score kept) so other players' history is unaffected.
- Take regular Postgres backups; the free tier does not back up for you.

## 9. Message Protocol (WebSocket)

All messages are JSON with a `type` field.

**Client → Server**
- `create_room { mode, friendly_fire, public, password? }`
- `join_room { code, password? }`
- `list_rooms { }`
- `reconnect { code, session_token }`
- `set_ready { ready }`
- `request_swap { }`
- `leave_room { }`
- `input { move, fire, tick }`

**Server → Client**
- `session { token }` (sent once on join)
- `room_list { rooms }`
- `room_state { code, mode, friendly_fire, players, teams, host, status }`
- `map { tiles }` (full map, on join and at the start of each round)
- `snapshot { tick, tanks, bullets, powerups, base_alive, lives, enemies_left }` (15 Hz; `tanks` includes players and AI)
- `map_event { kind, x, y }`
- `player_disconnected { name }` / `player_reconnected { name }`
- `player_eliminated { name }`
- `match_over { winner, scores }`
- `error { message }`

## 10. Anti-Cheat (Minimal)

The engine runs in the browser, so players can modify their copy. The server doesn't trust it.

- **The server decides.** Hits, scores, spawns, and eliminations come from the server's copy of the engine.
- **Input validation.** The server checks inputs against the rules and drops invalid ones.
- **Rate limiting.** Cap messages per second per connection, for both game input and login attempts.
- **Message size limit.** Reject oversized messages.
- **Name and username rules.** Enforced on the server, with a basic filter for obvious slurs.
- **Room password.** A basic gate, not security.
- **Host controls.** The host can kick players from the lobby before the match starts.

Single Player has no server, so there's nothing to cheat against there.

Skip for now: hardware ID tracking, client anti-tamper, automated ban systems.

## 11. Testing Plan

- **Engine:** Unit tests in `game-core` for movement, tank collisions, bullets, brick and steel destruction, enemy spawning, lives, respawn, invulnerability, power-up rules per mode (including an enemy collecting one), and win conditions.
- **Determinism:** Replays from seed plus input log produce the same final state.
- **Rounds:** A PvP round resets lives, score, and upgrades; best-of-3 ends at 2 wins.
- **Scoring:** The stored total equals the player's own contribution, not the team total; a team kill deducts the penalty.
- **Rooms:** Code generation, public list, password checks, rejecting joins after start, host transfer, swaps, empty-room cleanup.
- **Reconnect:** Works with the right token within 30 seconds; fails with a wrong token or after the window.
- **Accounts:** Sign-up, verification, password reset, login rate limiting, session expiry.
- **History:** A finished match appears in both players' histories, and totals update once.
- **Client:** Manual testing on Chrome, Firefox, and Safari.
- **Network:** Simulated latency, and several browser windows in one room.

## 12. Sound and Asset Credits

- Use only CC0 (public domain) or licenses that allow free use with credit (for example, CC-BY).
- Avoid licenses that forbid commercial use if the game might ever be monetized.
- Keep a `CREDITS.md` file in the repo, listing for every asset: name, author, source URL, and license.
- Show the same credits on the in-game credits page (Milestone 5).
- Good sources: Kenney.nl (CC0), OpenGameArt.org (check each license), Freesound.org (check each license).

## 13. Repository Layout

```
tank-in-city/
├─ game-core/            # Rust crate: all game rules (WASM + native)
│  ├─ src/
│  │  ├─ lib.rs
│  │  ├─ map.rs
│  │  ├─ tank.rs
│  │  ├─ bullet.rs
│  │  ├─ ai.rs
│  │  ├─ powerups.rs
│  │  ├─ rules.rs        # modes, lives, win conditions, scoring
│  │  └─ rng.rs
│  └─ tests/
├─ web/                  # Next.js frontend
│  ├─ app/               # routes (title, login, menu, lobby, game, profile, credits)
│  ├─ components/        # shadcn/ui components
│  └─ game/              # canvas renderer, input, prediction, interpolation
├─ server/               # Rust backend (Axum)
│  ├─ src/
│  │  ├─ main.rs
│  │  ├─ auth/           # accounts, sessions, email tokens
│  │  ├─ ws/             # WebSocket handling
│  │  ├─ rooms/          # codes, public list, lobby, teams, host, sessions
│  │  └─ db/             # SQLx queries
│  └─ migrations/
├─ assets/               # sprites, sounds, map data
└─ CREDITS.md
```

## 14. Tech Stack

| Layer | Choice |
|---|---|
| Game engine | Rust crate `game-core`, compiled to WASM (browser) and native (server) |
| Frontend | Next.js (React, TypeScript), Tailwind CSS |
| UI components | shadcn/ui |
| Rendering | HTML5 Canvas |
| Backend | Rust, Axum, Tokio |
| Real-time | WebSockets, JSON, 15 Hz snapshots |
| Database | PostgreSQL, SQLx |
| Email | Transactional email provider with a free tier (choose at Milestone 4) |

### Free deployment plan (launch)

Free tiers change often, and several limits matter for this game:
- **Sleep and cold starts:** Some free web services spin down after inactivity. A sleeping server drops live rooms and makes the first connection slow.
- **Free databases expire:** Some free Postgres plans delete data after 30 days or 90 days, or limit runtime.
- **WebSocket limits:** Not every free tier supports long-lived WebSocket connections well.
- **Restarts:** A reload or redeploy drops live matches. Warn players in-game; they can rejoin once the server is back. Rooms never survive a restart.

A workable launch setup:
- **Frontend:** Vercel free tier or Cloudflare Pages.
- **Backend:** A free-tier server host, or a free VM if one is available. Expect sleep or limits; players may need to wait for the first connection.
- **Database:** A free serverless Postgres such as Neon, which pauses when idle. Check its current limits before relying on it.
- **Email:** A free plan from a transactional provider, checked against current limits.

When players and history grow, move the backend to a small paid VPS (a few dollars per month). The Rust server is a single binary, so the move is simple.

Verify current free-tier limits on each provider's pricing page before launch.

### Logging and observability

Use Rust `tracing` with `tracing-subscriber`, JSON output in production. Levels: `info` for lifecycle, `warn` and `error` for problems. Tag each line with the room code or user id where relevant, so one match can be traced end to end.

**Log these (necessary):**
- Server start and stop, build version, and a config summary (never secrets).
- Room lifecycle: created, closed, code-collision retry, mode, player count.
- Match lifecycle: started, round started and ended, match ended, winner, duration.
- Player events: join, leave, kick, disconnect, reconnect (name plus room code). Never per-input.
- Auth events: sign-up, login success and failure, rate-limit trips, password reset, email verification. Log the user id, not the email or any token.
- Errors and warnings: panics, failed database writes, failed email sends, WebSocket protocol errors, oversized messages, expired reconnect windows.

**Never log (this is the part that would bury a server):**
- Per-tick game state or individual input messages.
- Passwords, raw session tokens, raw email tokens, or full email addresses.

## 15. Open Questions

1. **Email provider and hosting:** Pick these at Milestone 4, once you've checked current free limits.
2. **PvP match length:** The default is best-of-3; confirm the round count after playtesting.
3. **Power-up numbers:** Durations and armor scaling are starting values; tune after playtesting.
