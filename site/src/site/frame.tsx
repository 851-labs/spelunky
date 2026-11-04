// The frame every page of the site has but /play/'s (the game's own page, which a release ships: it's the game's
// whole window): the skip link, the header, the page's main and its footer, in one column whose left edge is the
// keyline (docs/wiki-style.md, "The keyline"), styled by site.css. The header is the logo (the way home), the
// site's sections, the one a page is in marked, and the wiki's search box (851-3994).
//
// The landing page's scripts mustn't have the wiki's code (test-bundle.mjs), so this imports none of it: a page
// gives the header its search box. The wiki's is Base UI's autocomplete (src/wiki/search-box.tsx); the landing
// page's is SiteSearch, the same form without the list, which goes to /wiki?q= as the autocomplete's does
// without a page chosen.
import { useLocation } from "@tanstack/react-router";
import type { ReactNode } from "react";

// The site's sections, in the header's order.
const SECTIONS = [
  { id: "play", href: "/play/", name: "Play" },
  { id: "wiki", href: "/wiki", name: "Wiki" },
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
        {SECTIONS.map(({ id, href, name }) => (
          <a key={id} href={href} aria-current={id === section ? (path === href ? "page" : "true") : undefined}>
            {name}
          </a>
        ))}
      </nav>
      {search}
    </header>
  );
}

// The footer's links on a page outside the wiki (the landing page, the 404 page), whose notices are its own.
export function SiteLinks() {
  return (
    <p>
      spelunky.ai &middot; <a href="/wiki">Wiki</a> &middot; <a href="https://github.com/851-labs/spelunky">GitHub</a>
    </p>
  );
}

// The header's search box where the wiki's code isn't: a plain form, as the wiki's is before its script runs. (On
// the 404 page, which has no scripts, an empty box goes to /wiki, the search's page, rather than nowhere.)
export function SiteSearch() {
  return (
    <form
      role="search"
      className="search-box"
      action="/wiki"
      method="get"
      // As the wiki's: an empty box goes nowhere.
      onSubmit={(e) => {
        if (!new FormData(e.currentTarget).get("q")?.toString().trim()) e.preventDefault();
      }}
    >
      <label className="visually-hidden" htmlFor="site-search">
        Search the wiki
      </label>
      <input id="site-search" type="search" name="q" placeholder="Search the wiki" autoComplete="off" spellCheck={false} />
      <button type="submit">Search</button>
    </form>
  );
}
