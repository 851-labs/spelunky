// The wiki's search boxes (docs/wiki-style.md, "The pipeline"): the header's, on every wiki page, which
// shows the best pages as you type in a list under it (Base UI's autocomplete: the arrow keys move through
// it, Enter opens one, Escape closes it) and otherwise goes to /wiki?q=; and the hub's, on /wiki, which
// lists every page that matches below it, the results that URL shows. The index (src/wiki/search.ts) is
// loaded the first time either is used, so a page that's only read never loads it.
import { Autocomplete } from "@base-ui/react/autocomplete";
import { Field } from "@base-ui/react/field";
import { Input } from "@base-ui/react/input";
import { useHydrated } from "@tanstack/react-router";
import { useCallback, useEffect, useId, useMemo, useRef, useState } from "react";
import { loadOnce, never } from "./load-once";
import { matchingSections, queryWords, searchWiki, wordMatches, words } from "./search";
import type { SearchDoc, SearchIndex } from "./search";
import type { Option, SearchPopup } from "./search-popup";

// How many pages the header's list shows.
const SUGGESTIONS = 8;

// The index, from its own chunk. The Worker never searches, so its bundle needn't carry it.
const loadSearchIndex = loadOnce(() => (import.meta.env.SSR ? never() : import("virtual:wiki/search-index")).then((module) => module.index));

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

// The header's list, from its own chunk (search-popup.tsx), loaded with the index.
const loadPopup = loadOnce(() => (import.meta.env.SSR ? never() : import("./search-popup")).then((module) => module.SearchPopup));

export const searchPath = (query: string) => `/wiki?${new URLSearchParams({ q: query })}`;

export const plural = (n: number) => (n === 1 ? "1 page" : `${n} pages`);

// The header's box: Base UI's autocomplete, whose list (search-popup.tsx) is the best pages and then all the
// results on /wiki. Without a page chosen, Enter submits the form, which works without scripts too.
export function SearchBox() {
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const form = useRef<HTMLFormElement>(null);
  const { index, failed, load: loadIndex } = useSearchIndex();
  const [Popup, setPopup] = useState<typeof SearchPopup | null>(null);
  const load = () => {
    loadIndex();
    if (!Popup) loadPopup().then((popup) => setPopup(() => popup), () => {});
  };
  const trimmed = query.trim();
  const hits = useMemo(() => (index && trimmed ? searchWiki(index, trimmed) : []), [index, trimmed]);
  const options: Option[] = hits.length === 0 ? [] : [...hits.slice(0, SUGGESTIONS).map((hit) => ({ href: `/wiki/${hit.doc.slug}`, hit })), { href: searchPath(trimmed), hit: null }];
  const status = !trimmed ? "" : failed ? "Search couldn't load." : !index ? "Searching…" : hits.length === 0 ? `No pages match “${trimmed}”.` : `${plural(hits.length)} found.`;
  const shown = open && trimmed !== "" && Popup !== null;

  return (
    <Autocomplete.Root
      items={options}
      // The index has searched already.
      filter={null}
      itemToStringValue={(option: Option) => option.href}
      value={query}
      onValueChange={(value) => {
        setQuery(value);
        load();
      }}
      // Not before the list's code is here, so the box never says it shows a list it hasn't got.
      open={shown}
      onOpenChange={setOpen}
    >
      <Field.Root
        render={
          <form
            ref={form}
            role="search"
            className="search-box"
            action="/wiki"
            method="get"
            onSubmit={(e) => {
              if (!trimmed) e.preventDefault();
            }}
          />
        }
      >
        <Field.Label className="visually-hidden">Search the wiki</Field.Label>
        <Autocomplete.Input
          type="search"
          name="q"
          placeholder="Search the wiki"
          autoComplete="off"
          spellCheck={false}
          // Back in the box, its list again.
          onFocus={() => {
            load();
            setOpen(true);
          }}
          onPointerEnter={load}
        />
        <button type="submit">Search</button>
        {/* Out loud, how many pages match: always there, as a live region must be to be heard. */}
        <Autocomplete.Status className="visually-hidden">{shown ? status : ""}</Autocomplete.Status>
      </Field.Root>
      {Popup && <Popup anchor={form} options={options} status={status} query={trimmed} total={hits.length} />}
    </Autocomplete.Root>
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
      <Field.Root render={<form role="search" className="search-box search-box-hub" action="/wiki" method="get" onSubmit={(e) => e.preventDefault()} />}>
        <Field.Label id={`${id}-label`}>Search the wiki</Field.Label>
        <Input
          type="search"
          name="q"
          placeholder="Snake, shotgun, the Mines…"
          autoComplete="off"
          spellCheck={false}
          aria-controls={`${id}-results`}
          value={shown}
          onValueChange={(value) => {
            setTyped(value);
            onQuery(value);
            load();
          }}
          onFocus={load}
        />
      </Field.Root>
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
export function Highlight({ text, query }: { text: string; query: string }) {
  const wanted = queryWords(query);
  // Split on words (with their accents, which words() drops), which split() keeps at the odd indexes.
  return text.split(/([\p{L}\p{M}\p{N}]+)/u).map((part, i) =>
    i % 2 === 1 && words(part).some((word) => wordMatches(word, wanted)) ? <mark key={i}>{part}</mark> : part,
  );
}
