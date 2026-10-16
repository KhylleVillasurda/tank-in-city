import Link from "next/link";

/**
 * Title screen: log in, create an account, or play as a guest
 * (plan section 3). Accounts arrive in Milestone 4; guest play works now.
 */
export default function Title() {
  return (
    <main className="mx-auto flex min-h-dvh max-w-xl flex-col justify-center gap-8 p-8">
      <div className="boot">
        {/* 32px, not a Tailwind step: Press Start 2P is drawn on an 8px grid, so
            whole multiples of 8 are the sizes that stay crisp. No font-bold —
            the face has one weight and a synthetic bold blurs the pixels. */}
        <h1 className="text-[32px] leading-snug tracking-tight">
          Tank In City
        </h1>
        <p className="mt-3 text-zinc-400">
          Top-down tank battles. Endless arcade against the AI.
        </p>
      </div>

      <div className="boot flex flex-col gap-3">
        <p className="blink text-[10px] text-emerald-400">&gt; insert coin</p>
        <Link
          className="border border-emerald-600 bg-emerald-600 px-4 py-3 text-center text-xs text-black hover:bg-emerald-500 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-emerald-500"
          href="/menu"
        >
          Play as guest
        </Link>
        <Disabled label="Log in" note="accounts arrive in Milestone 4" />
        <Disabled label="Create an account" note="accounts arrive in Milestone 4" />
      </div>
    </main>
  );
}

function Disabled({ label, note }: { label: string; note: string }) {
  // Wraps on purpose: the pixel face is wider than the monospace it replaced,
  // so the note drops to its own line instead of squeezing the label.
  return (
    <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1 border border-zinc-800 px-4 py-3 text-zinc-600">
      <span className="whitespace-nowrap text-xs">{label}</span>
      <span className="text-xs">{note}</span>
    </div>
  );
}
