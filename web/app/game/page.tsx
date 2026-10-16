import Link from "next/link";

import GameCanvas from "@/components/GameCanvas";

export default function GamePage() {
  return (
    <main className="flex min-h-dvh flex-col items-center justify-center">
      <Link className="mt-6 text-sm text-zinc-500 hover:text-zinc-300" href="/menu">
        &larr; main menu
      </Link>
      <GameCanvas />
    </main>
  );
}
