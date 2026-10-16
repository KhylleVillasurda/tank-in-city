# Sound

What sounds this game needs, where to get them, and what happens to them once
they are here. Sound is Milestone 5 in the plan; this file is the shopping list
and the rulebook, written before the first file arrives so nothing has to be
guessed later.

Plan references: §1 ("do not use sprites or sounds ripped from the original
game"), §12 ("Sound and Asset Credits"), §13 (asset folders).

## The one rule

**Never take audio out of the original Battle City.** Not the ROM, not a
YouTube rip, not a "soundfont". That is the mistake most clones make and it is
the only thing here that is actually illegal. Everything else on this page is
free to use.

## The files to get

Twelve sounds cover the whole game. Names in the table are the names to save
them as; if a pack uses different names, keep its names and say so — the
loader is written last, not first.

| Save as | Sound | Used when |
|---|---|---|
| `shoot-player.ogg` | short blip | you fire |
| `shoot-enemy.ogg` | slightly different blip | an enemy fires |
| `hit-brick.ogg` | dull knock | a bullet breaks a brick |
| `hit-steel.ogg` | sharp ping, nothing breaks | a bullet hits steel |
| `enemy-destroyed.ogg` | small explosion | you kill an enemy |
| `player-destroyed.ogg` | bigger explosion | you die |
| `powerup-appear.ogg` | flash or sparkle | the bonus enemy starts flashing |
| `powerup-pickup.ogg` | rising blip | you drive over a power-up |
| `base-destroyed.ogg` | long crash | the base is lost, run over |
| `stage-clear.ogg` | short jingle | stage finished |
| `game-over.ogg` | down-turning jingle | run over |
| `menu-select.ogg` | click | title screen button |

Optional, if you find good ones:

- `engine-loop.ogg` — a short rumble, looped while you are alive.
- `music-stage.ogg` — one background track. This is the file most likely to be
  big and most likely to need a credit, so treat it separately.

Formats: `.ogg` is what Kenney packs ship and it plays in every current browser.
`.wav` works but is 5–10x bigger; `.mp3` plays everywhere except very old
Firefox. Keep each sound under about 50 KB and under about 1 second, except the
jingle tracks.

## What arrived (first batch)

Six files, measured rather than eyeballed. This is what came in the door:

| Delivered as | Length | Peak level | Size | Verdict |
|---|---|---|---|---|
| `shoot.wav` | 0.14 s | −14.1 dB | 12 KB | far too quiet |
| `explosion.wav` | 0.70 s | −5.0 dB | 121 KB | slightly quiet |
| `tank_moving.wav` | 0.42 s | −9.7 dB | 73 KB | engine loop, one of two |
| `tank_moving_by_kenny.ogg` | 3.99 s | 0.0 dB | 67 KB | engine loop, the other |
| `level_cleared.wav` | 13.7 s | −0.0 dB | 2.6 MB | 13.7 s against a 2 s pause |
| `Battle_by_PerituneMaterial_8bitRPG.mp3` | 2 min 11 s | −0.1 dB | 3.1 MB | the music |

Good news to start with: every file begins immediately, with no silence at the
front, so nothing on this list would have felt late when played.

### What was done about it

| File | Change |
|---|---|
| `shoot.ogg` | +11.1 dB — it was 14 dB under everything else, which is roughly a quarter as loud, and would have vanished under the music |
| `explosion.ogg` | +2.0 dB — it sat about 5 dB under the others |
| `tank_moving.ogg`, `level_cleared.ogg`, the music | re-encoded: the WAVs to Ogg Vorbis, the music to 96 kbps. The folder went from 5.9 MB to 1.8 MB |
| the stage-clear pause | no longer a timer at all: the game waits for the player to click or press Enter (see below) |

Peaks are now close together — shoot −1.6 dB, explosion −0.9 dB, the rest
between −0.0 and −7.7 — so nothing is buried and nothing jumps out. The one
thing not settled is the **balance** between the shot and the explosion, because
that is a by-ear judgement and there is no speaker here. If shooting turns out
to feel harsh in play, `shoot.ogg` wants a few dB off, and that is one number.

The 13.7 s jingle is the reason the stage-clear screen changed. A 2 s
auto-advance cut the sound off six times over, so rather than trim the sound the
game now **stops and waits**: clearing the last enemy shows "Stage clear — click,
or press Enter, to carry on" and the next stage loads when the player says so.
The whole jingle plays, for as long as the player wants to hear it. That is an
engine change, not an audio one, and it is written up in `docs/milestone-2.md`.

The engine loop is wired to `tank_moving_by_kenny.ogg`, chosen because Kenney is
CC0 and has no extraction clause — the one sound that plays constantly is the one
worth keeping free of strings. The two files are **different sounds**, not
duplicates, and one has to be picked by ear, so this is provisional: swapping
`engine:` in `web/game/audio.ts` is a one-line change if the other one sounds
better in play.

### What is covered, and what is still missing

| Sound | File |
|---|---|
| you fire | `shoot.ogg` |
| an enemy fires | `shoot.ogg`, pitched down — or find a separate blip |
| enemy destroyed, you destroyed, base destroyed | `explosion.ogg` (one sound for all three, or pitch it) |
| engine loop, while alive | `tank_moving.ogg` or `tank_moving_by_kenny.ogg` |
| stage cleared | `level_cleared.ogg` |
| background music | `Battle_by_PerituneMaterial_8bitRPG.ogg` |

Still to find, of the twelve: brick hit, steel hit, power-up appear, power-up
pickup, game over, menu select. Six files, not nine, because of the reuse above.

## Where to get them

Best first, and all of these allow free use. The licence column matters more
than the sound quality.

| Source | Licence | Notes |
|---|---|---|
| [Kenney.nl](https://kenney.nl/assets) | CC0 | No credit needed. Browse packs by name: *Impact Sounds*, *Digital Audio*, *UI Audio*, *Sci-Fi Sounds*, *Music Jingles*. Start here. |
| [sfxr.me](https://sfxr.me/) / [bfxr.net](https://www.bfxr.net/) | yours | Generators, not libraries. Battle City's sound is NES square-wave beeps, so these fit exactly. No download, no licence, nothing to credit. |
| [Freesound.org](https://freesound.org/) | mixed | Needs a free account. Set the licence filter to CC0. Sort by rating, not relevance. |
| [OpenGameArt.org](https://opengameart.org/) | mixed | Good retro packs. Licences vary per item, check each. |
| [itch.io](https://itch.io/sound-effects) | mixed | Search `free sound pack retro` or `8 bit sfx free`. Read each pack's licence line. |
| [Pixabay](https://pixabay.com/sound-effects/) | own terms | Free, no credit. Cannot redistribute the raw files, shipping them inside a game is fine. |

Music only: [Pixabay Music](https://pixabay.com/music/) (no credit),
[Free Music Archive](https://freemusicarchive.org/) (filter CC0),
[Incompetech](https://incompetech.com/music/royalty-free/) (credit required),
[Uppbeat](https://uppbeat.io/) (free tier wants a credit).

## How to search

Search for the **game event**, not a mood. Queries that return results:

```
8 bit explosion
retro laser shot
chiptune pickup
arcade game over jingle
8 bit engine loop
brick break impact
```

Then set the licence filter before downloading, not after.

Three licences, plainly:

| Licence | What it means |
|---|---|
| CC0 | Use it, no credit needed. Prefer this. |
| CC-BY | Use it, but the author must be named in `CREDITS.md` and on the credits page. |
| Non-commercial | Skip it, unless the game is certain never to earn money. |

The licences actually in use, and the two conditions in them that a web game
does not satisfy, are written up in `CREDITS.md` under "Licence notes" — that is
the file the plan designates for it, so it is not repeated here.

## Where the files go

`web/public/audio/` — the browser needs to fetch them, and Next.js serves
anything under `web/public/` directly. A file at `web/public/audio/shoot-player.ogg`
is requested as `/audio/shoot-player.ogg`.

**Not** the root `assets/` folder. That folder is for data the Rust crate bakes
in at compile time (the stage `.txt` files). Sound is a browser thing, and
`game-core` is not allowed to know the browser exists.

The exception is the untouched downloads, which sit in `assets/` alongside
`assets/stages/` and are **not committed**: two of the three licences forbid
redistributing the unmodified file, and a git repository is redistribution. So
the raw files stay local as the source for re-converting a sound, and only the
converted files that actually ship are committed. The rule is in `.gitignore`
with a pointer here.

## After each file arrives

Add one row to `CREDITS.md` — name, author, source URL, licence. The table is
already there. The same list feeds the in-game credits page in Milestone 5
(plan §12), so it only has to be written once.

## How the sounds get triggered

Written, in `web/game/audio.ts`, called from the frame callback in
`web/game/engine.ts` next to `render.ts`. There is **no engine change**: the
browser already receives a snapshot every frame, so every event is the
difference between this frame and the last one.

| Event | How the browser knows | Sound |
|---|---|---|
| A shot was fired | that side's bullets in flight went up (owner 0 is yours) | `shoot.ogg`, pitched to 0.75 for an enemy |
| An enemy was destroyed | the score went up — a kill is the only thing that scores | `explosion.ogg` |
| You were destroyed | a life went | `explosion.ogg`, pitched to 0.7 |
| The base was lost | `base_alive` turned off | `explosion.ogg`, pitched to 0.5 |
| Stage cleared | state became 1 — and the screen waits for the player, so the jingle is never cut off | `level_cleared.ogg` |
| You are driving | your x or y changed since the last frame | `tank_moving_*.ogg`, looped, fading in and out |
| A run is on screen | the first frame after the context unlocks | the music, looped |

Three details worth keeping:

- **Shots are counted per side, not per slot.** The bullet list is compacted, so
  a slot number says nothing about who owns it. A shot is only possible when
  that side has a slot free, so its count always goes up when one is fired.
- **A kill is read off the score**, not off the enemy count. The enemy count also
  moves when a wave spawns; the score only moves when something dies.
- **The engine loop follows the wheels, not being alive.** A drone that runs the
  whole time you are alive is a noise, not a sound effect. It fades in and out
  over 50 ms so a loop that starts or stops does not click.

Still to add, when the files exist: brick hit, steel hit, power-up appear,
power-up pickup, game over, menu select.

Two known limits, both fine to accept for now:

- Two shots in the same frame produce one sound, not two. At 60 frames a second
  with the cooldowns this game uses, that basically never happens.
- Brick and steel hits both just mean "a bullet disappeared without a kill". If
  the two sounds need to differ, that needs the tile map re-read, which is
  extra work for a small gain.

### The browser will not make a noise until you touch it

Chrome, Firefox and everything else refuse to start audio until the player has
interacted with the page. A run nobody has clicked or keyed into is silent, and
that is not a bug. So the context is created suspended as soon as the game page
loads, every file is fetched and decoded up front, and the first key press or
click resumes it. In practice the first arrow key a player presses is the
gesture, and by then the sounds are already decoded and waiting.

If there is ever no sound at all, check in this order: a **mute tab** or a
system volume first (the game cannot detect either), then whether the files
return 200 at `/audio/<name>.ogg`, then `npm run check-audio`.

## Checking the sound without speakers

`npm run check-audio` runs `scripts/check-audio.mjs`. It swaps the audio device
for a stub that records what was asked to play, drives the real wasm, and asserts
that each sound fires on the right event: silence while the context is locked,
the shot on firing, the pitched-down shot for enemies, the explosion on a kill
and a lower one on a lost life or the base, the jingle on a stage clear, and the
engine loop starting and stopping with the wheels.

It cannot say whether a sound is *good*, or whether the mix is right. Nothing on
this machine can. That is what a play test is for.
