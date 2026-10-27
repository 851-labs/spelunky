// The header's search box's list (src/wiki/search-box.tsx): Base UI's autocomplete popup, of the best pages
// and then all the results on /wiki. Its own chunk, loaded with the search index the first time the box is
// used, so the pages' bundle carries only the box. Each page is a link, which Enter follows as a click does.
import { Autocomplete } from "@base-ui/react/autocomplete";
import type { RefObject } from "react";
import type { SearchHit } from "./search";
import { Highlight, plural } from "./search-box";

// A page in the list, or (with no hit) all the results.
export interface Option {
  href: string;
  hit: SearchHit | null;
}

// Under the box, its right edge at the form's (the button's), and inside the viewport. Without a list, the
// status, which says why (the box's status says it out loud).
export function SearchPopup({
  anchor,
  options,
  status,
  query,
  total,
}: {
  anchor: RefObject<HTMLFormElement | null>;
  options: Option[];
  status: string;
  query: string;
  total: number;
}) {
  return (
    <Autocomplete.Portal>
      <Autocomplete.Positioner className="search-positioner" anchor={anchor} side="bottom" align="end" sideOffset={8} collisionPadding={16}>
        <Autocomplete.Popup className="search-popup">
          {options.length === 0 && <p className="search-note">{status}</p>}
          <Autocomplete.List aria-label="Pages">
            {(option: Option) => (
              // Not a stop for Tab: the arrow keys move through the list.
              <Autocomplete.Item key={option.href} value={option} tabIndex={-1} render={<a href={option.href} />}>
                {option.hit ? (
                  <>
                    <span className="search-title">
                      <Highlight text={option.hit.doc.title} query={query} />
                    </span>
                    {option.hit.doc.category && <span className="search-category"> · {option.hit.doc.category}</span>}
                    <span className="search-lede">{option.hit.doc.lede}</span>
                  </>
                ) : (
                  <span className="search-all">All {plural(total)} matching “{query}”</span>
                )}
              </Autocomplete.Item>
            )}
          </Autocomplete.List>
        </Autocomplete.Popup>
      </Autocomplete.Positioner>
    </Autocomplete.Portal>
  );
}
