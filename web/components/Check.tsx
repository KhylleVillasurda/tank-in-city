"use client";

/**
 * A checkbox the site draws itself.
 *
 * Kept as a real `<input type="checkbox">` with `appearance-none`: the browser
 * still owns the label, the space bar, the focus and the form value, and only
 * the square itself is replaced with one that matches the rest of the page.
 */
export default function Check({
  checked,
  onChange,
  children,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  children: React.ReactNode;
}) {
  return (
    <label className="flex cursor-pointer items-center gap-3 text-sm text-zinc-400 select-none">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
        className="size-4 shrink-0 cursor-pointer appearance-none border border-zinc-700 bg-black checked:border-emerald-500 checked:bg-emerald-600 focus-visible:outline focus-visible:outline-1 focus-visible:outline-offset-2 focus-visible:outline-emerald-600"
      />
      {children}
    </label>
  );
}
