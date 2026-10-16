"use client";

import { useEffect, useRef, useState } from "react";

import { Game } from "@/game/engine";
import { loadCore, type Constants, type Snapshot } from "@/game/wasm";

/** How often the HUD text is allowed to re-render, in milliseconds. */
const HUD_INTERVAL_MS = 100;

/** Remembers the music switch, so it survives a reload. */
const MUSIC_PREF_KEY = "tank-in-city.music";

type Hud = { snapshot: Snapshot; paused: boolean; seed: number };

/**
 * `?seed=1234` pins a run, so a play report can be replayed exactly. Anything
 * that is not plainly digits falls back to a fresh random run.
 */
function seedFromUrl(): number | undefined {
  const raw = new URLSearchParams(window.location.search).get("seed");
  if (!raw || !/^\d{1,10}$/.test(raw)) return undefined;
  return Number(raw) >>> 0;
}

export default function GameCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const gameRef = useRef<Game | null>(null);
  const [consts, setConsts] = useState<Constants | null>(null);
  const [hud, setHud] = useState<Hud | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [musicOff, setMusicOff] = useState(false);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    let game: Game | null = null;
    let disposed = false;
    let hudAt = 0;
    let latest: Hud | null = null;

    loadCore()
      .then(({ core, consts: loaded }) => {
        if (disposed) return;
        setConsts(loaded);
        game = new Game(core, loaded, canvas, (snapshot, paused, seed) => {
          latest = { snapshot, paused, seed };
          const now = performance.now();
          if (now - hudAt < HUD_INTERVAL_MS) return;
          hudAt = now;
          setHud(latest);
        });
        gameRef.current = game;
        // The switch is the player's, so it outlives the page: read it here
        // rather than during render, and hand it to the audio before the first
        // frame plays anything.
        const off = savedMusicOff();
        setMusicOff(off);
        game.setMusicOff(off);
        game.start(seedFromUrl());
      })
      .catch((err: unknown) => setError(String(err)));

    return () => {
      disposed = true;
      gameRef.current = null;
      game?.stop();
    };
  }, []);

  const toggleMusic = () => {
    const off = !musicOff;
    setMusicOff(off);
    gameRef.current?.setMusicOff(off);
    try {
      window.localStorage.setItem(MUSIC_PREF_KEY, off ? "off" : "on");
    } catch {
      // A browser that refuses storage just does not remember the choice.
    }
  };

  const snapshot = hud?.snapshot;

  return (
    <div className="flex flex-col items-center gap-4 p-6">
      <div className="flex w-full max-w-[832px] items-center justify-between text-sm">
        <span>
          Score <span className="text-emerald-400">{snapshot?.players[0]?.score ?? 0}</span>
        </span>
        <span>
          Stage <span className="text-emerald-400">{snapshot?.stage ?? 1}</span>
        </span>
        <span>
          Enemies <span className="text-emerald-400">{snapshot?.enemiesLeft ?? 0}</span>
        </span>
        <span className="flex items-center gap-3">
          <Lives left={snapshot?.players[0]?.lives ?? 0} />
          <button
            type="button"
            aria-pressed={!musicOff}
            onClick={toggleMusic}
            className="border border-zinc-700 px-2 py-1 text-[10px] text-zinc-400 hover:border-emerald-600 hover:text-emerald-400 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-500"
          >
            {musicOff ? "Music off" : "Music on"}
          </button>
        </span>
      </div>

      <div className="relative">
        <canvas
          ref={canvasRef}
          onClick={() => gameRef.current?.clicked()}
          className="border border-zinc-700"
        />

        {error && (
          <Overlay>
            <span className="text-red-400">{error}</span>
          </Overlay>
        )}
        {!error && consts && snapshot && (
          <Overlay>
            {snapshot.state === consts.stateGameOver ? (
              <div className="space-y-2">
                <p className="text-[24px]">Game over</p>
                <p className="text-zinc-300">
                  {snapshot.players[0]?.score ?? 0} points, reached stage {snapshot.stage}
                </p>
                <p className="text-zinc-500">Press R to run again</p>
                <p className="text-xs text-zinc-600">
                  seed {hud?.seed} &mdash;{" "}
                  <a
                    className="underline hover:text-zinc-400"
                    href={`/game?seed=${hud?.seed}`}
                  >
                    replay this run
                  </a>
                </p>
              </div>
            ) : hud?.paused ? (
              <p className="text-[16px]">Paused</p>
            ) : snapshot.state === consts.stateStageClear ? (
              <div className="space-y-2">
                <p className="text-[16px]">Stage clear</p>
                <p className="text-zinc-400">Click, or press Enter, to carry on</p>
              </div>
            ) : null}
          </Overlay>
        )}
      </div>

      <p className="text-sm text-zinc-500">
        Arrows or WASD to drive, Space to fire, Enter to carry on after a stage,
        P to pause, R to restart.
      </p>
    </div>
  );
}

function Overlay({ children }: { children: React.ReactNode }) {
  return (
    <div className="pointer-events-none absolute inset-0 grid place-items-center bg-black/70 text-center">
      {children}
    </div>
  );
}

/** The music switch, as it was left last time. Never throws on a hostile browser. */
function savedMusicOff(): boolean {
  try {
    return window.localStorage.getItem(MUSIC_PREF_KEY) === "off";
  } catch {
    return false;
  }
}

/**
 * The run's remaining lives, as Battle City showed them: one box per life rather
 * than a number. The boxes are the whole meaning, so they carry one label for a
 * screen reader instead of being announced as a row of empty spans.
 */
function Lives({ left }: { left: number }) {
  return (
    <span className="flex items-center gap-2">
      <span className="text-zinc-500">Lives</span>
      <span
        role="img"
        aria-label={`${left} ${left === 1 ? "life" : "lives"} left`}
        className="flex items-center gap-1"
      >
        {Array.from({ length: left }, (_, i) => (
          <span
            key={i}
            aria-hidden
            className="h-3 w-3 border border-emerald-500 bg-emerald-600"
          />
        ))}
      </span>
    </span>
  );
}
