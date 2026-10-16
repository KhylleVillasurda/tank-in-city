"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";

import { guestName } from "@/game/guest";
import {
  MODE_LABELS,
  TEAM_LABELS,
  type Mode,
  type RoomRow,
  type RoomState,
  type ServerMessage,
  serverUrl,
} from "@/game/lobby";

const MODES = Object.keys(MODE_LABELS) as Mode[];

const BUTTON =
  "border border-zinc-700 px-3 py-2 font-pixel text-xs text-zinc-300 hover:border-emerald-600 hover:text-emerald-400 disabled:border-zinc-900 disabled:text-zinc-700";
const GO =
  "border border-emerald-600 bg-emerald-600 px-4 py-3 text-center font-pixel text-xs text-black hover:bg-emerald-500";
const FIELD = "border border-zinc-800 bg-black px-3 py-2 text-sm text-zinc-200";

/**
 * The lobby (plan section 3). It speaks the protocol in `game/lobby.ts` and
 * decides nothing itself: the server owns the room, the sides and the start
 * rules, and pushes a fresh `room_state` after every change.
 *
 * A match cannot run yet — the engine is single player — so "start" moves the
 * room to `playing` and that is as far as this slice goes.
 */
export default function Lobby() {
  const socket = useRef<WebSocket | null>(null);
  const [name, setName] = useState("");
  const [where, setWhere] = useState("");
  const [connected, setConnected] = useState(false);
  const [note, setNote] = useState("");
  const [room, setRoom] = useState<RoomState | null>(null);
  const [rooms, setRooms] = useState<RoomRow[]>([]);

  const [mode, setMode] = useState<Mode>("1v1");
  const [friendlyFire, setFriendlyFire] = useState(false);
  const [isPublic, setIsPublic] = useState(true);
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");

  // Generated on the client so the name is not baked into the HTML, and sent
  // with create and join: the server has no accounts yet (Milestone 4).
  useEffect(() => setName(guestName()), []);

  useEffect(() => {
    const url = serverUrl();
    setWhere(url);
    const ws = new WebSocket(url);
    socket.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setNote("");
    };
    ws.onmessage = (event) => {
      const msg = JSON.parse(event.data) as ServerMessage;
      if (msg.type === "room_state") setRoom(msg);
      else if (msg.type === "room_list") setRooms(msg.rooms);
      else setNote(msg.message);
    };
    ws.onerror = () => setNote(`could not reach the game server at ${url}`);
    ws.onclose = () => setConnected(false);

    return () => {
      ws.onclose = null;
      ws.close();
    };
  }, []);

  const send = useCallback((msg: object) => {
    const ws = socket.current;
    if (ws?.readyState === WebSocket.OPEN) ws.send(JSON.stringify(msg));
  }, []);

  // Plan section 3: "The list refreshes automatically."
  useEffect(() => {
    if (!connected || room) return;
    send({ type: "list_rooms" });
    const timer = setInterval(() => send({ type: "list_rooms" }), 3000);
    return () => clearInterval(timer);
  }, [connected, room, send]);

  const join = (wanted: string) =>
    send({
      type: "join_room",
      name,
      code: wanted.trim().toUpperCase(),
      password: password || null,
    });

  const leave = () => {
    send({ type: "leave_room" });
    // Nobody gets a message back for leaving — they are no longer in the room —
    // so the client clears its own state. That is right either way: if the
    // leave was refused, there was nothing to be in.
    setRoom(null);
    setNote("");
  };

  const mine = room?.players.find((p) => p.name === name);

  return (
    <main className="mx-auto flex min-h-dvh max-w-2xl flex-col justify-center gap-6 p-8">
      <div className="boot">
        <h1 className="font-pixel text-[24px] leading-snug tracking-tight">Lobby</h1>
        <p className="mt-3 text-sm text-zinc-500">
          Playing as <b className="text-zinc-300">{name || "guest"}</b>{" "}
          <span className={connected ? "text-emerald-400" : "text-zinc-500"}>
            {connected ? "connected" : "connecting"}
          </span>
        </p>
        <p className="mt-1 text-xs text-zinc-600">{where}</p>
      </div>

      {note && (
        <p className="border border-amber-700 px-4 py-3 text-sm text-amber-400">{note}</p>
      )}

      {room ? (
        <div className="boot flex flex-col gap-4">
          <div className="flex flex-wrap items-baseline justify-between gap-2">
            <p className="font-pixel text-[16px] tracking-widest">{room.code}</p>
            <p className="text-sm text-zinc-500">
              {MODE_LABELS[room.mode]} · friendly fire{" "}
              {room.friendly_fire ? "on" : "off"} · {room.status}
            </p>
          </div>

          {[...new Set(room.players.map((p) => p.team))].map((team) => (
            <div key={team}>
              <p className="text-xs text-zinc-500">{TEAM_LABELS[team]}</p>
              <ul className="mt-1 flex flex-col gap-1">
                {room.players
                  .filter((p) => p.team === team)
                  .map((p) => (
                    <li
                      key={p.name}
                      className="flex items-center justify-between border border-zinc-800 px-3 py-2"
                    >
                      <span className="text-sm text-zinc-300">
                        {p.name}
                        {p.name === room.host && (
                          <span className="text-zinc-500"> host</span>
                        )}
                        {p.name === name && <span className="text-zinc-500"> you</span>}
                      </span>
                      <span
                        className={
                          p.ready ? "text-xs text-emerald-400" : "text-xs text-zinc-600"
                        }
                      >
                        {p.ready ? "ready" : "not ready"}
                      </span>
                    </li>
                  ))}
              </ul>
            </div>
          ))}

          <div className="flex flex-wrap gap-3">
            <button
              className={BUTTON}
              onClick={() => send({ type: "set_ready", ready: !mine?.ready })}
            >
              {mine?.ready ? "Not ready" : "Ready"}
            </button>
            {room.host === name && room.status === "waiting" && (
              <button className={BUTTON} onClick={() => send({ type: "start_match" })}>
                Start match
              </button>
            )}
            <button className={BUTTON} onClick={leave}>
              Leave
            </button>
          </div>

          {room.status === "playing" && (
            <p className="text-sm text-zinc-500">
              The match loop is not built yet, so this is where a room stops.
            </p>
          )}
        </div>
      ) : (
        <div className="boot flex flex-col gap-6">
          <div className="flex flex-col gap-3">
            <p className="font-pixel text-[10px] text-zinc-400">Host a room</p>
            <label className="flex items-center justify-between border border-zinc-800 px-3 py-2 text-sm text-zinc-400">
              Mode
              <select
                className={FIELD}
                value={mode}
                onChange={(e) => setMode(e.target.value as Mode)}
              >
                {MODES.map((m) => (
                  <option key={m} value={m}>
                    {MODE_LABELS[m]}
                  </option>
                ))}
              </select>
            </label>
            <label className="flex items-center gap-3 text-sm text-zinc-400">
              <input
                type="checkbox"
                checked={isPublic}
                onChange={(e) => setIsPublic(e.target.checked)}
              />
              List it publicly
            </label>
            <label className="flex items-center gap-3 text-sm text-zinc-400">
              <input
                type="checkbox"
                checked={friendlyFire}
                onChange={(e) => setFriendlyFire(e.target.checked)}
              />
              Friendly fire (Free For All forces it on)
            </label>
            <input
              className={FIELD}
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Room password (optional)"
            />
            <button
              className={GO}
              onClick={() =>
                send({
                  type: "create_room",
                  name,
                  mode,
                  friendly_fire: friendlyFire,
                  public: isPublic,
                  password: password || null,
                })
              }
            >
              Host room
            </button>
          </div>

          <div className="flex flex-col gap-3">
            <p className="font-pixel text-[10px] text-zinc-400">Join a room</p>
            <div className="flex gap-2">
              <input
                className={`${FIELD} flex-1 tracking-widest`}
                value={code}
                maxLength={5}
                onChange={(e) => setCode(e.target.value.toUpperCase())}
                placeholder="CODE"
              />
              <button className={BUTTON} onClick={() => join(code)}>
                Join
              </button>
            </div>
            <p className="text-xs text-zinc-600">
              The password box above is used here too.
            </p>
          </div>

          <div className="flex flex-col gap-2">
            <p className="font-pixel text-[10px] text-zinc-400">Public rooms</p>
            {rooms.length === 0 ? (
              <p className="text-sm text-zinc-600">
                None right now. This refreshes every 3 seconds.
              </p>
            ) : (
              <ul className="flex flex-col gap-1">
                {rooms.map((r) => (
                  <li
                    key={r.code}
                    className="flex items-center justify-between gap-2 border border-zinc-800 px-3 py-2"
                  >
                    <span className="text-sm">
                      <b className="font-pixel text-[10px] tracking-widest">{r.code}</b>{" "}
                      <span className="text-zinc-500">
                        {r.host} · {MODE_LABELS[r.mode]} · {r.players} playing ·{" "}
                        {r.status}
                      </span>
                    </span>
                    <button
                      className={BUTTON}
                      disabled={r.status !== "waiting"}
                      onClick={() => join(r.code)}
                    >
                      Join
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>
      )}

      <Link className="text-sm text-zinc-500 hover:text-zinc-300" href="/menu">
        &larr; back to menu
      </Link>
    </main>
  );
}
