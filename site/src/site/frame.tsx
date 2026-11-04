// The frame every page of the site has but /play/'s (the game's own page, which a release ships: it's the game's
// whole window): the skip link, the header, the page's main and its footer, in one column whose left edge is the
// keyline (docs/wiki-style.md, "The keyline"), styled by site.css. The header is the logo (the way home), the
// site's sections, the one a page is in marked, and the wiki's search box (851-3994).
//
// The landing page's scripts mustn't have the wiki's code up front (test-bundle.mjs), so this imports none of it
// but load-once (a few lines): a page gives the header its search box. The wiki's is Base UI's autocomplete
// (src/wiki/search-box.tsx); the landing page's is SiteSearch, the same form, which loads the autocomplete when
// it's first used.
import { useLocation } from "@tanstack/react-router";
import { useEffect, useRef, useState } from "react";
import type { CompositionEvent, ReactNode } from "react";
import { loadOnce, never } from "../wiki/load-once";

// The site's sections, in the header's order. Play opens the game in a tab of its own, so the page a player came
// from (the wiki, the landing page) stays open beside it.
const SECTIONS = [
  { id: "play", href: "/play/", name: "Play", newTab: true },
  { id: "wiki", href: "/wiki", name: "Wiki", newTab: false },
  // The native apps (src/routes/download.tsx), in this tab: it's a page of the site's, not the game.
  { id: "download", href: "/download", name: "Download", newTab: false },
] as const;

export type Section = (typeof SECTIONS)[number]["id"];

export function SiteFrame({
  section,
  search,
  footer,
  after,
  children,
}: {
  section: Section | null;
  search: ReactNode;
  footer: ReactNode;
  // What's out of the main column's flow, between it and the footer (the wiki's hover cards).
  after?: ReactNode;
  children: ReactNode;
}) {
  return (
    <div className="site">
      <a className="skip" href="#content">
        Skip to content
      </a>
      <SiteHeader section={section} search={search} />
      <main id="content">{children}</main>
      {after}
      <footer className="site-footer">{footer}</footer>
    </div>
  );
}

// The section's link is the current page's ("page") on the section's own page, and otherwise says the page is
// in it ("true"), which a screen reader reads as "current" too.
function SiteHeader({ section, search }: { section: Section | null; search: ReactNode }) {
  const path = useLocation({ select: (location) => location.pathname });
  return (
    <header className="site-header">
      <a href="/" className="site-home" aria-current={path === "/" ? "page" : undefined}>
        <img src="/media/logo.png" width="160" height="36" alt="Spelunky" />
      </a>
      <nav className="site-nav" aria-label="Site">
        {SECTIONS.map(({ id, href, name, newTab }) => (
          <a
            key={id}
            href={href}
            aria-current={id === section ? (path === href ? "page" : "true") : undefined}
            {...(newTab ? { target: "_blank", rel: "noopener" } : {})}
          >
            {name}
            {newTab && <span className="visually-hidden"> (opens in a new tab)</span>}
          </a>
        ))}
      </nav>
      {search}
    </header>
  );
}

// The footer's links on a page outside the wiki (the landing page, /download, /license, the 404 page), whose notices
// are its own: the license on every one of them, as the wiki's footer has it.
export function SiteLinks() {
  return (
    <p>
      spelunky.ai &middot; <a href="/wiki">Wiki</a> &middot; <a href="/license">License</a> &middot;{" "}
      <a href="https://github.com/851-labs/spelunky">GitHub</a>
    </p>
  );
}

// The header's search box where the wiki's code isn't up front (the landing page, the 404 page): a plain form, as
// the wiki's is before its script runs, which goes to /wiki?q=. The first time it's focused or pressed, it loads
// the wiki's box (Base UI's autocomplete, src/wiki/search-box.tsx) in a chunk of its own, which takes its place
// with what's typed, the selection and the focus (search-swap.tsx), so it lists pages as you type there too
// (851-4004) without the landing page's scripts carrying it (test-bundle.mjs). All but loading it is the chunk's.
// Without scripts (the 404 page has none), it stays the form, where an empty box goes to /wiki, the search's page,
// rather than nowhere.
const loadSwap = loadOnce(() => (import.meta.env.SSR ? never() : import("./search-swap")).then((module) => module.swap));

export function SiteSearch() {
  const [box, setBox] = useState<ReactNode>(null);
  const form = useRef<HTMLFormElement>(null);
  // Once (again only if the chunk didn't load); its cancel, if the form goes first.
  const started = useRef(false);
  const cancel = useRef<() => void>(undefined);
  const upgrade = () => {
    if (started.current) return;
    started.current = true;
    loadSwap().then(
      (swap) => (cancel.current = swap(form.current, setBox)),
      () => (started.current = false),
    );
  };
  // Focused before the page was hydrated, when no handler saw it.
  useEffect(() => {
    if (form.current?.contains(document.activeElement)) upgrade();
    return () => cancel.current?.();
  }, []);
  // An input method's composition under way, which the swap waits out: it may start before the chunk is here.
  const composing = (on: boolean) => (e: CompositionEvent<HTMLFormElement>) => e.currentTarget.toggleAttribute("data-composing", on);
  return (
    box ?? (
      <form
        ref={form}
        role="search"
        className="search-box"
        action="/wiki"
        method="get"
        // As the wiki's: an empty box goes nowhere.
        onSubmit={(e) => {
          if (!new FormData(e.currentTarget).get("q")?.toString().trim()) e.preventDefault();
        }}
        onFocus={upgrade}
        onPointerDown={upgrade}
        onCompositionStart={composing(true)}
        onCompositionEnd={composing(false)}
      >
        <label className="visually-hidden" htmlFor="site-search">
          Search the wiki
        </label>
        <input id="site-search" type="search" name="q" placeholder="Search the wiki" autoComplete="off" spellCheck={false} />
        <button type="submit">Search</button>
      </form>
    )
  );
}
