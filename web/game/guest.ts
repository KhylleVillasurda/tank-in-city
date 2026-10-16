/**
 * Guest identity (plan section 2).
 *
 * A guest gets a generated name, unique within a room. There are no rooms
 * until Milestone 3, so for now the name is only shown back to the player,
 * but it is generated the way the plan specifies: `SilentTank42`.
 */

const ADJECTIVES = [
  "Silent",
  "Rusty",
  "Iron",
  "Brave",
  "Quick",
  "Lucky",
  "Grim",
  "Steel",
  "Frozen",
  "Grey",
];

const KEY = "tic.guest.name";

export function guestName(): string {
  const stored = read();
  if (stored) return stored;

  const name = `${pick(ADJECTIVES)}Tank${10 + Math.floor(Math.random() * 90)}`;
  write(name);
  return name;
}

function read(): string | null {
  try {
    return sessionStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function write(name: string) {
  try {
    sessionStorage.setItem(KEY, name);
  } catch {
    // Private mode: the name just does not survive navigation.
  }
}

function pick<T>(items: T[]): T {
  return items[Math.floor(Math.random() * items.length)];
}
