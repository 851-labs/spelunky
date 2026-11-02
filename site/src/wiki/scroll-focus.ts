// A box that scrolls sideways when what's in it is wider than the column: a body's table on a phone, a GML
// excerpt's long lines, a room's picture. A keyboard scrolls only what has focus, and the box holds nothing
// focusable, so without a tab stop of its own its cut-off part can't be reached without a pointer (axe's
// scrollable-region-focusable: 851-3892). So while it overflows, the box is a tab stop, a region named
// `label`; while it doesn't, it's neither, so the tables that fit add no tab stops or landmarks. Native
// scrolling, not Base UI's ScrollArea, which would draw every one's scrollbar itself: this only names a box
// the browser already scrolls. The pre-render can't measure, so the box gets them once the page hydrates.
import { useEffect, useRef, useState, type HTMLAttributes, type RefObject } from "react";

export function useScrollFocus<T extends HTMLElement>(label: string | ((box: T) => string)): [RefObject<T | null>, HTMLAttributes<T>] {
  const ref = useRef<T>(null);
  const [name, setName] = useState<string | null>(null);
  // The latest label, so a function given inline doesn't re-run the effect each render.
  const latest = useRef(label);
  latest.current = label;
  useEffect(() => {
    const box = ref.current;
    if (!box) return;
    const measure = () => {
      const label = latest.current;
      setName(box.scrollWidth > box.clientWidth ? (typeof label === "string" ? label : label(box)) : null);
    };
    measure();
    // The box's width (the window's), and its content's (a font or picture that loads).
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    for (const child of box.children) observer.observe(child);
    return () => observer.disconnect();
  }, []);
  return [ref, name === null ? {} : { tabIndex: 0, role: "region", "aria-label": name }];
}

// A body's table's name: its section's heading's, "Defeating table", or "Table" before the first heading.
export function tableLabel(box: HTMLElement): string {
  for (let at: Element | null = box; at && !at.matches("article, main, body"); at = at.parentElement) {
    for (let before = at.previousElementSibling; before; before = before.previousElementSibling) {
      const heading = before.matches("h2, h3") ? before : before.querySelector(":scope.heading > :is(h2, h3)");
      const text = heading?.textContent?.trim();
      if (text) return `${text} table`;
    }
  }
  return "Table";
}
