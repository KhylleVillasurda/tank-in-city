"use client";

import { useEffect, useId, useRef, useState, type ReactNode } from "react";

type Option<T extends string> = { value: T; label: ReactNode };

/**
 * A dropdown the site draws itself.
 *
 * A native `<select>` popup is drawn by the operating system, so it cannot be
 * styled to match anything. The cost of replacing it is the behaviour a native
 * one gives away for free, so that is what the rest of this file is: the listbox
 * roles, the arrow keys, Enter, Escape, and a click outside to dismiss.
 */
export default function Select<T extends string>({
  label,
  value,
  options,
  onChange,
}: {
  label: string;
  value: T;
  options: readonly Option<T>[];
  onChange: (value: T) => void;
}) {
  const [open, setOpen] = useState(false);
  const [at, setAt] = useState(0);
  const id = useId();
  const box = useRef<HTMLDivElement>(null);

  // Clicking anywhere else dismisses it, the way a native dropdown behaves.
  useEffect(() => {
    if (!open) return;
    const away = (event: PointerEvent) => {
      if (!box.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, [open]);

  const current = options.find((o) => o.value === value) ?? options[0];
  /** Where the highlight lands when the list opens: on what is selected now. */
  const currentAt = Math.max(0, options.findIndex((o) => o.value === value));
  const openAt = () => {
    setAt(currentAt);
    setOpen(true);
  };
  const pick = (v: T) => {
    onChange(v);
    setOpen(false);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step !== 0) {
      event.preventDefault();
      if (open) setAt((a) => (a + step + options.length) % options.length);
      else openAt();
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      if (open && options[at]) pick(options[at].value);
      else openAt();
    } else if (event.key === "Escape" && open) {
      event.preventDefault();
      setOpen(false);
    }
  };

  return (
    <div ref={box} className="relative">
      <button
        type="button"
        role="combobox"
        aria-expanded={open}
        aria-controls={id}
        aria-activedescendant={open ? `${id}-${at}` : undefined}
        onClick={() => (open ? setOpen(false) : openAt())}
        onKeyDown={onKeyDown}
        className="flex w-full items-center justify-between border border-zinc-800 px-3 py-2 text-sm text-zinc-400 focus-visible:border-emerald-600 focus-visible:outline-none"
      >
        {label}
        <span className="flex items-center gap-2 text-zinc-200">
          {current?.label}
          <span aria-hidden className="text-zinc-600">
            {open ? "▲" : "▼"}
          </span>
        </span>
      </button>

      {open && (
        <ul
          id={id}
          role="listbox"
          aria-label={label}
          className="absolute inset-x-0 top-full z-10 flex flex-col border border-zinc-800 bg-black"
        >
          {options.map((o, i) => (
            <li
              key={o.value}
              id={`${id}-${i}`}
              role="option"
              aria-selected={o.value === value}
              onPointerEnter={() => setAt(i)}
              // pointerdown, not click: it fires before focus moves, so the
              // highlighted row is not lost to a blur on the way through.
              onPointerDown={(event) => {
                event.preventDefault();
                pick(o.value);
              }}
              className={`cursor-pointer px-3 py-2 text-sm ${
                i === at ? "bg-zinc-900 text-emerald-400" : "text-zinc-300"
              }`}
            >
              {o.label}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
