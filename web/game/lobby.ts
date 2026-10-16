/**
 * The lobby protocol (plan section 9), as the browser sees it.
 *
 * The server is the only place the rules live. This file is just the shape of
 * the messages, the labels the lobby shows, and where to find the server —
 * nothing here decides anything.
 */

export type Mode = "1v1" | "2v2" | "3v3" | "free_for_all" | "coop";
export type Team = "a" | "b" | "none";
export type Status = "waiting" | "playing";

export interface Player {
  name: string;
  team: Team;
  ready: boolean;
}

export interface RoomState {
  type: "room_state";
  code: string;
  mode: Mode;
  friendly_fire: boolean;
  host: string;
  status: Status;
  players: Player[];
}

export interface RoomRow {
  code: string;
  host: string;
  mode: Mode;
  players: number;
  status: Status;
}

export type ServerMessage =
  | RoomState
  | { type: "room_list"; rooms: RoomRow[] }
  | { type: "error"; message: string };

/** The wire names come from the server, so this only adds the display text. */
export const MODE_LABELS: Record<Mode, string> = {
  "1v1": "1v1",
  "2v2": "2v2",
  "3v3": "3v3",
  free_for_all: "Free For All",
  coop: "Co-op vs AI",
};

/** Which side a player is on. Free For All and Co-op both end up with one. */
export const TEAM_LABELS: Record<Team, string> = {
  a: "Side A",
  b: "Side B",
  none: "Free for all",
};

/**
 * Where the game server is.
 *
 * `?server=` wins, so a tunnel can be pointed at without a rebuild: a quick
 * tunnel hands out a new address every run (docs/sharing.md).
 *
 * Otherwise: behind a tunnel the page and the socket share a host, because the
 * tunnel config routes `/ws` to the game server. Locally the web is on 3000 and
 * the server on 8080, which is what `cd server && cargo run` starts.
 */
export function serverUrl(): string {
  const asked = new URLSearchParams(location.search).get("server");
  if (asked) return asked;
  if (location.protocol === "https:") return `wss://${location.host}/ws`;
  return `ws://${location.hostname}:8080/ws`;
}
