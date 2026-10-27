// The wiki's search boxes (docs/wiki-style.md, "The pipeline"): the header's, on every wiki page, which
// shows the best pages as you type in a list under it (a combobox: the arrow keys move through it, Enter
// opens one, Escape closes it) and otherwise goes to /wiki?q=; and the hub's, on /wiki, which lists every
// page that matches below it, the results that URL shows. The index (src/wiki/search.ts) is loaded the first
// time either is used, so a page that's only read never loads it.
import { useHydrated } from "@tanstack/react-router";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import type { KeyboardEvent } from "react";
import { matchingSections, queryWords, searchWiki, wordMatches, words } from "./search";
import type { SearchDoc, SearchIndex } from "./search";

// How many pages the header's list shows.
const SUGGESTIONS = 8;

let loading: Promise<SearchIndex> | null = null;

// The index, from its own chunk: loaded once, and tried again after a failure (the network, say).
function loadSearchIndex(): Promise<SearchIndex> {
  // The Worker never searches, so its bundle needn't carry the index.
  if (import.meta.env.SSR) return new Promise(() => {});
  return (loading ??= import("virtual:wiki/search-index").then(
    (module) => module.index,
    (error) => {
      loading = null;
      throw error;
    },
  ));
}

// The index (null until it's loaded), whether it failed to load, and `load` to load it, or try again.
function useSearchIndex() {
  const [index, setIndex] = useState<SearchIndex | null>(null);
  const [state, setState] = useState<"idle" | "loading" | "failed">("idle");
  const load = useCallback(() => {
    if (index || state === "loading") return;
    setState("loading");
    loadSearchIndex().then(setIndex, () => setState("failed"));
  }, [index, state]);
  return { index, failed: state === "failed", load };
}

export const searchPath = (query: string) => `/wiki?${new URLSearchParams({ q: query })}`;

const plural = (n: number) => (n === 1 ? "1 page" : `${n} pages`);

// The header's box.
export function SearchBox() {
  const id = useId();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(-1);
  const { index, failed, load } = useSearchIndex();
  const trimmed = query.trim();
  const hits = useMemo(() => (index && trimmed ? searchWiki(index, trimmed) : []), [index, trimmed]);
  // The best pages, then all the results on /wiki.
  const options = hits.length === 0 ? [] : [...hits.slice(0, SUGGESTIONS).map((hit) => `/wiki/${hit.doc.slug}`), searchPath(trimmed)];
  const shown = open && trimmed !== "";
  const expanded = shown && options.length > 0;
  const status = !trimmed ? "" : failed ? "Search couldn't load." : !index ? "Searching…" : hits.length === 0 ? `No pages match “${trimmed}”.` : `${plural(hits.length)} found.`;

  function onKeyDown(e: KeyboardEvent<HTMLInputElement>) {
    // An input method's keys are its own until it's done composing.
    if (e.nativeEvent.isComposing) return;
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      if (options.length === 0) return;
      e.preventDefault();
      setOpen(true);
      const step = e.key === "ArrowDown" ? 1 : -1;
      // From the input, down is the first option and up the last; past either end is the input again.
      setActive((i) => (!expanded ? (step === 1 ? 0 : options.length - 1) : i + step < -1 ? options.length - 1 : i + step >= options.length ? -1 : i + step));
    } else if (e.key === "Enter" && expanded && active >= 0) {
      e.preventDefault();
      window.location.assign(options[active]);
    } else if (e.key === "Escape" && shown) {
      // The first Escape closes the list; the next one clears the box, as a search input does.
      e.preventDefault();
      setOpen(false);
      setActive(-1);
    }
  }

  return (
    <form
      role="search"
      className="search-box"
      action="/wiki"
      method="get"
      onSubmit={(e) => {
        if (!trimmed) e.preventDefault();
      }}
    >
      <label htmlFor={`${id}-input`} className="visually-hidden">
        Search the wiki
      </label>
      <input
        id={`${id}-input`}
        type="search"
        name="q"
        placeholder="Search the wiki"
        autoComplete="off"
        spellCheck={false}
        role="combobox"
        aria-autocomplete="list"
        aria-expanded={expanded}
        aria-controls={`${id}-list`}
        aria-activedescendant={expanded && active >= 0 ? `${id}-${active}` : undefined}
        value={query}
        onChange={(e) => {
          setQuery(e.target.value);
          setOpen(true);
          setActive(-1);
          load();
        }}
        onFocus={() => {
          load();
          setOpen(true);
        }}
        onPointerEnter={load}
        onBlur={() => {
          setOpen(false);
          setActive(-1);
        }}
        onKeyDown={onKeyDown}
      />
      <button type="submit">Search</button>
      {/* Pressing in the list doesn't take the focus from the box, so it stays open for the click. */}
      <div className="search-popup" hidden={!shown} onMouseDown={(e) => e.preventDefault()}>
        <ul id={`${id}-list`} role="listbox" aria-label="Pages">
          {expanded &&
            options.map((href, i) => {
              // The last option is the results page's.
              const hit = i < options.length - 1 ? hits[i] : null;
              return (
                <li key={href} id={`${id}-${i}`} role="option" aria-selected={i === active}>
                  <a href={href} tabIndex={-1}>
                    {hit ? (
                      <>
                        <span className="search-title">
                          <Highlight text={hit.doc.title} query={trimmed} />
                        </span>
                        {hit.doc.category && <span className="search-category"> · {hit.doc.category}</span>}
                        <span className="search-lede">{hit.doc.lede}</span>
                      </>
                    ) : (
                      <span className="search-all">All {plural(hits.length)} matching “{trimmed}”</span>
                    )}
                  </a>
                </li>
              );
            })}
        </ul>
        {shown && options.length === 0 && <p className="search-note">{status}</p>}
      </div>
      <p role="status" className="visually-hidden">
        {shown ? status : ""}
      </p>
    </form>
  );
}

// The hub's box and its results: what /wiki?q= shows. `query` is the URL's, which `onQuery` changes.
export function WikiSearch({ query, onQuery }: { query: string; onQuery: (query: string) => void }) {
  const id = useId();
  // The pre-rendered /wiki has no query: the box and the results follow the URL's once the page is
  // hydrated, so that hydrating doesn't change what the server rendered.
  const hydrated = useHydrated();
  // What's typed, once something is: the box's own, as the URL catches up with each change a moment later
  // (a box showing the URL's would put the caret back at the end on every key).
  const [typed, setTyped] = useState<string | null>(null);
  const shown = typed ?? (hydrated ? query : "");
  const trimmed = shown.trim();
  const { index, failed, load } = useSearchIndex();
  useEffect(() => {
    if (trimmed && !failed) load();
  }, [trimmed, failed, load]);
  const hits = useMemo(() => (index && trimmed ? searchWiki(index, trimmed) : []), [index, trimmed]);
  const status = !trimmed ? "" : failed ? "Search couldn't load." : !index ? "Searching…" : hits.length === 0 ? `No pages match “${trimmed}”.` : `${plural(hits.length)} ${hits.length === 1 ? "matches" : "match"} “${trimmed}”.`;

  return (
    <section className="wiki-search" aria-labelledby={`${id}-label`}>
      <form role="search" className="search-box search-box-hub" action="/wiki" method="get" onSubmit={(e) => e.preventDefault()}>
        <label htmlFor={`${id}-input`} id={`${id}-label`}>
          Search the wiki
        </label>
        <input
          id={`${id}-input`}
          type="search"
          name="q"
          placeholder="Snake, shotgun, the Mines…"
          autoComplete="off"
          spellCheck={false}
          aria-controls={`${id}-results`}
          value={shown}
          onChange={(e) => {
            setTyped(e.target.value);
            onQuery(e.target.value);
            load();
          }}
          onFocus={load}
        />
      </form>
      <p role="status" className={trimmed ? "search-status" : "visually-hidden"}>
        {status}
      </p>
      <ol id={`${id}-results`} className="search-results" hidden={hits.length === 0}>
        {hits.map((hit) => (
          <Result key={hit.doc.slug} doc={hit.doc} query={trimmed} />
        ))}
      </ol>
    </section>
  );
}

// A result on /wiki: the page's title, category and lede, and the sections whose headings match.
function Result({ doc, query }: { doc: SearchDoc; query: string }) {
  const sections = matchingSections(doc, query);
  return (
    <li>
      <a href={`/wiki/${doc.slug}`} className="search-title">
        <Highlight text={doc.title} query={query} />
      </a>
      {doc.category && <span className="search-category"> · {doc.category}</span>}
      <p>
        <Highlight text={doc.lede} query={query} />
      </p>
      {sections.length > 0 && (
        <ul className="search-sections" aria-label={`Sections of ${doc.title}`}>
          {sections.map(([anchor, title]) => (
            <li key={anchor}>
              <a href={`/wiki/${doc.slug}#${anchor}`}>
                <Highlight text={title} query={query} />
              </a>
            </li>
          ))}
        </ul>
      )}
    </li>
  );
}

// The text with the words the query matches marked.
function Highlight({ text, query }: { text: string; query: string }) {
  const wanted = queryWords(query);
  // Split on words (with their accents, which words() drops), which split() keeps at the odd indexes.
  return text.split(/([\p{L}\p{M}\p{N}]+)/u).map((part, i) =>
    i % 2 === 1 && words(part).some((word) => wordMatches(word, wanted)) ? <mark key={i}>{part}</mark> : part,
  );
}
