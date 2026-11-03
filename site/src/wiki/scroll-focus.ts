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
  // The latest label, so a function given inline doesn't re-run the effect each render; a string that changes
  // does, renaming the box.
  const latest = useRef(label);
  latest.current = label;
  const text = typeof label === "string" ? label : null;
  useEffect(() => {
    const box = ref.current;
    if (!box) return;
    const measure = () => {
      const label = latest.current;
      // Only a box that scrolls: a wider table spills out of its box at desktop widths, where it doesn't.
      const scrolls = /auto|scroll/.test(getComputedStyle(box).overflowX) && box.scrollWidth > box.clientWidth;
      setName(scrolls ? (typeof label === "string" ? label : label(box)) : null);
    };
    measure();
    // The box's width (the window's), its content's (a picture that loads), and a font that loads, which can
    // widen an excerpt's lines without resizing any box an observer can watch (its code is inline).
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    for (const child of box.children) observer.observe(child);
    document.fonts?.addEventListener("loadingdone", measure);
    return () => {
      observer.disconnect();
      document.fonts?.removeEventListener("loadingdone", measure);
    };
  }, [text]);
  return [ref, name === null ? {} : { tabIndex: 0, role: "region", "aria-label": name }];
}

// A body's table's name: its section's heading's, "Defeating table", or "Table" before the first heading.
const HEADING = "h2, h3, h4";
export function tableLabel(box: HTMLElement): string {
  for (let at: Element | null = box; at && !at.matches("article, main, body"); at = at.parentElement) {
    for (let before = at.previousElementSibling; before; before = before.previousElementSibling) {
      const heading = before.matches(HEADING) ? before : before.querySelector(`:scope.heading > :is(${HEADING})`);
      const text = heading?.textContent?.trim();
      if (text) return `${text} table`;
    }
  }
  return "Table";
}
