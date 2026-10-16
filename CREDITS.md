# Credits

Every asset shipped with Tank In City is listed here: name, author, source URL
and licence, per plan section 12. Only CC0 or free-use-with-credit licences
(CC-BY and similar) go in this list.

First assets arrived with the sound pack. Two of the three sources carry a
condition worth knowing about — see "Licence notes" below.

## Assets

| Asset | Author | Source | Licence |
|---|---|---|---|
| `shoot.ogg` | _uploader to fill in_ | samplefocus.com — _sample page URL to fill in_ | Sample Focus Standard Licence |
| `explosion.ogg` | _uploader to fill in_ | samplefocus.com — _sample page URL to fill in_ | Sample Focus Standard Licence |
| `level_cleared.ogg` | _uploader to fill in_ | samplefocus.com — _sample page URL to fill in_ | Sample Focus Standard Licence |
| `tank_moving.ogg` | _uploader to fill in_ | samplefocus.com — _sample page URL to fill in_ | Sample Focus Standard Licence |
| `tank_moving_by_kenny.ogg` | Kenney | [kenney.nl](https://kenney.nl/assets) — _pack name to fill in_ | CC0 1.0 |
| `Battle_by_PerituneMaterial_8bitRPG.ogg` | PeriTune (Sei Mutsuki / むつき醒) | [peritune.com](https://peritune.com/) — _track page URL to fill in_ | PeriTune terms (see below) |
| `web/app/fonts/press-start-2p.ttf` | Cody "CodeMan38" Boisclair (cody@zone38.net) | [fontspace](https://www.fontspace.com/press-start-2p-font-f11591) | SIL Open Font License 1.1 |
| Ammo-8 — 8 colours, `web/components/pixel/ammo-8.css` | Ammo | [lospec.com/palette-list/ammo-8](https://lospec.com/palette-list/ammo-8) | palette, credited to its author (see below) |
| Eulbink — 7 colours, `web/components/pixel/eulbink.css` | Inkpendude | [lospec.com/palette-list/eulbink](https://lospec.com/palette-list/eulbink) | as above |
| Cryptic Ocean — 6 colours, `web/components/pixel/cryptic-ocean.css` | Klafooty | [lospec.com/palette-list/cryptic-ocean](https://lospec.com/palette-list/cryptic-ocean) | as above |

The four names left blank are the ones worth pasting in when you have the pages
open. Uploader names are not required by any of these licences, but the plan
lists an author for every asset, and it takes a minute while the page is still
in your history.

Every file above is shipped as Ogg Vorbis. Three of them were WAVs and one was
an MP3 as downloaded; the WAVs were re-encoded and the music was re-encoded to
96 kbps, which took the folder from 5.9 MB to 1.8 MB. The untouched downloads
stay in `assets/` next to `assets/stages/`, and they are deliberately **not
committed**: two of the three licences forbid redistributing the unmodified file,
and a repository is redistribution. If a sound needs redoing, convert it again
from there.

## Licence notes

All three sources allow a free game and all three allow commercial use. Two of
them have a condition that a web game does not satisfy, and it is the same
condition in both cases: **do not hand the raw sound file to the public.**

- **Sample Focus** ([full licence](https://samplefocus.com/license)) grants free
  use in "games, apps, and multimedia projects", commercial included, and needs
  **no credit**. Two catches. First, it forbids making a sound "available in a
  complete, archived, downloadable, or readily extractable format". Second, the
  sounds are uploaded by users: Sample Focus reviews and screens them, but
  states plainly that "no review can rule out every possible third-party
  claim". So it is a permissive licence, not a guaranteed-clean one.
- **PeriTune** ([terms, in Japanese](https://peritune.com/about/)) grants free
  personal and commercial use, and explicitly allows editing and looping, which
  is what a game needs. Credit is **optional** for tracks published from March
  2026, but tracks published before February 2026 stay under **CC BY 4.0**,
  where credit is required — so crediting PeriTune either way satisfies both.
  The relevant catch: a song may be bundled in a **free** game, but a **paid**
  product must make the file "non-extractable by encryption or similar means".
- **Kenney** is CC0: no conditions, nothing required. This is the only source of
  the three with no strings at all.
- **Lospec palettes** (Ammo-8, Eulbink, Cryptic Ocean) are lists of colours, not
  files: what ships is eight hex values per theme, as CSS variables in
  `web/components/pixel/`. So the "do not hand out the raw file" conditions above
  do not apply to them. Lospec publishes each palette with its author's name and
  no licence text of its own, so the honest entry is the author and the source
  URL; they are credited here because plan §12 asks for an author on every asset,
  and because a palette is somebody's work. A palette used only as a *reference*
  for a role scheme would need no credit at all — these are the shipped colours.
- **Press Start 2P** is under the SIL Open Font License 1.1. Free to use and
  embed, including commercially, and no visible credit is required — the licence
  lives beside the file in `web/app/fonts/OFL.txt` and it is credited here
  anyway, because plan §12 asks for an author on every asset. It is vendored
  rather than fetched from a font service at build time, so a build never needs
  the network. The copy shipped is the full TTF from fontspace (v2.0, 82 KB),
  not the Latin subset from Google Fonts (v3.0, 12 KB) that it replaced.

**What this means in practice.** For a free game, credit them and carry on.
Serving audio from `/audio/*.ogg` is the only way a browser can play it, so the
"no direct download" clause is in tension with any web game, not just this one —
and the clause is clearly aimed at people reselling the raw files, not at games.
The honest position is: fine now, revisit if the game is ever sold. If it is,
the paid-product rule bites, and the clean answer is to swap the four Sample
Focus sounds and the music for CC0 equivalents (Kenney has equivalents for all
four sounds), or generate them with `sfxr`.

## Credits page

Plan §12 also asks for these same credits on the in-game credits page. That
page arrives in Milestone 5 and reads from this list.
