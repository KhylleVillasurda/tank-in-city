"use client";

import Link from "next/link";
import { useEffect, useState } from "react";

import { guestName } from "@/game/guest";

/**
 * Main menu (plan section 3). Host and Join need rooms (Milestone 3) and
 * Profile needs accounts (Milestone 4).
 */
export default function Menu() {
  const [name, setName] = useState("");

  // Generated on the client so the name is not baked into the HTML.
  useEffect(() => setName(guestName()), []);

  return (
    <main className="mx-auto flex min-h-dvh max-w-xl flex-col justify-center gap-8 p-8">
      <div className="boot">
        <h1 className="font-pixel text-[24px] leading-snug tracking-tight">
          Main menu
        </h1>
        <p className="mt-3 text-sm text-zinc-500">
          Playing as <b className="text-zinc-300">{name || "guest"}</b>
        </p>
      </div>

      <div className="boot flex flex-col gap-3">
        <Link
          className="border border-emerald-600 bg-emerald-600 px-4 py-3 text-center font-pixel text-xs text-black hover:bg-emerald-500"
          href="/game"
        >
          Single Player
        </Link>
        <Disabled label="Host Room" note="Milestone 3" />
        <Disabled label="Join Room" note="Milestone 3" />
        <Disabled label="Profile" note="Milestone 4" />
      </div>

      <Link className="text-sm text-zinc-500 hover:text-zinc-300" href="/">
        &larr; back to title
      </Link>
    </main>
  );
}

function Disabled({ label, note }: { label: string; note: string }) {
  return (
    <div className="flex items-center justify-between border border-zinc-800 px-4 py-3 text-zinc-600">
      <span className="font-pixel text-xs">{label}</span>
      <span className="text-xs">{note}</span>
    </div>
  );
}
