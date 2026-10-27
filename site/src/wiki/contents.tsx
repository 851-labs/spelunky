// A page's table of contents, numbered as Wikipedia's (1, 1.1...), each entry the section's stable anchor,
// in two places, the CSS showing one (wiki.css, "The table of contents"):
// - in the body, before its first section: a box at medium widths, and on a phone a "Contents" disclosure
//   (Base UI's Collapsible), closed, so the page's text comes first;
// - on a wide screen, a rail left of the article that stays on screen as it scrolls, the section being read
//   marked (aria-current), as Grokipedia's.
// A short page doesn't need one.
import { Collapsible } from "@base-ui/react/collapsible";
import { useEffect, useRef, useState } from "react";
import type { Section } from "./build/compile";

// The line the section being read is the last heading above, as a fraction of the viewport's height from the
// top: a heading becomes the current one a little before it reaches the top.
const READING_LINE = 0.3;
// How far above the viewport the scroll-spy's observer looks (rootMargin's top), so that a heading being
// above the line, however far, is its intersecting: a fast scroll past one still crosses it.
const ABOVE = 1e6;

interface Entry {
  section: Section;
  children: Section[];
}

function tree(sections: Section[]): Entry[] | null {
  if (sections.length < 3) return null;
  const entries: Entry[] = [];
  for (const section of sections) {
    const parent = entries.at(-1);
    if (section.depth === 3 && parent?.section.depth === 2) parent.children.push(section);
    else entries.push({ section, children: [] });
  }
  return entries;
}

function List({ entries, active }: { entries: Entry[]; active?: string | null }) {
  const entry = (section: Section) => (
    <a href={`#${section.id}`} aria-current={section.id === active ? "location" : undefined}>
      {section.title}
    </a>
  );
  return (
    <ol>
      {entries.map(({ section, children }) => (
        <li key={section.id}>
          {entry(section)}
          {children.length > 0 && (
            <ol>
              {children.map((child) => (
                <li key={child.id}>{entry(child)}</li>
              ))}
            </ol>
          )}
        </li>
      ))}
    </ol>
  );
}

// The body's. Its panel stays in the page while it's closed (keepMounted), and the CSS shows it whatever its
// state but on a phone, so the box at medium widths is the same HTML with the toggle hidden.
export function Contents({ sections }: { sections: Section[] }) {
  const entries = tree(sections);
  if (!entries) return null;
  return (
    <Collapsible.Root render={<nav className="toc" id="toc" aria-labelledby="toc-title" />}>
      <p className="toc-title" id="toc-title">
        Contents
      </p>
      <Collapsible.Trigger className="toc-toggle">Contents</Collapsible.Trigger>
      <Collapsible.Panel className="toc-panel" keepMounted>
        <List entries={entries} />
      </Collapsible.Panel>
    </Collapsible.Root>
  );
}

// The rail's. The section being read is the last whose heading is above the reading line; at the end of the
// page, where the last sections can't scroll up to it, the last whose heading is on screen. An
// IntersectionObserver says when a heading crosses the line or the footer comes into view, rather than a
// listener on every scroll. Before the first heading, none is marked. Only the marking changes, which moves
// nothing (the CSS gives every entry the marker's room).
export function ContentsRail({ sections }: { sections: Section[] }) {
  const entries = tree(sections);
  const [active, setActive] = useState<string | null>(null);
  const ref = useRef<HTMLElement>(null);
  const ids = sections.map((section) => section.id).join(" ");

  useEffect(() => {
    const headings = ids
      .split(" ")
      .map((id) => document.getElementById(id))
      .filter((heading) => heading !== null);
    const footer = document.querySelector(".wiki-footer");
    if (headings.length === 0) return;
    let atEnd = false;
    const update = () => {
      const line = window.innerHeight * READING_LINE;
      let current: string | null = null;
      for (const heading of headings) {
        const top = heading.getBoundingClientRect().top;
        if (top <= line || (atEnd && top < window.innerHeight)) current = heading.id;
      }
      setActive(current);
    };
    const crossings = new IntersectionObserver(update, {
      rootMargin: `${ABOVE}px 0px ${-(1 - READING_LINE) * 100}% 0px`,
    });
    for (const heading of headings) crossings.observe(heading);
    const end = new IntersectionObserver(([entry]) => {
      atEnd = entry.isIntersecting;
      update();
    });
    if (footer) end.observe(footer);
    return () => {
      crossings.disconnect();
      end.disconnect();
    };
  }, [ids]);

  // A rail longer than the screen scrolls on its own: keep the marked entry in it.
  useEffect(() => {
    const rail = ref.current;
    const link = rail?.querySelector("[aria-current]");
    if (!rail || !link || rail.scrollHeight <= rail.clientHeight) return;
    const box = rail.getBoundingClientRect();
    const { top, bottom } = link.getBoundingClientRect();
    if (top < box.top) rail.scrollTop -= box.top - top + box.height / 4;
    else if (bottom > box.bottom) rail.scrollTop += bottom - box.bottom + box.height / 4;
  }, [active]);

  if (!entries) return null;
  return (
    <nav className="toc-rail" id="toc-rail" aria-labelledby="toc-rail-title" ref={ref}>
      <p className="toc-title" id="toc-rail-title">
        Contents
      </p>
      <List entries={entries} active={active} />
    </nav>
  );
}
