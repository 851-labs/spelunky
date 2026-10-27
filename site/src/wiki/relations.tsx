// A page's relations, as compile.ts resolved them (docs/wiki-style.md, "Relations"): a relation's links,
// which the infobox's Relations group shows too, and the See also and What links here sections.
import { Cite, H2, WikiLink } from "./components";
import type { PageMeta, RelationLink } from "./pages";

// A relation's links, one a line: each a link (or a red link), its note and its citations.
export function RelationLinks({ links }: { links: RelationLink[] }) {
  return (
    <ul className="relation-links">
      {links.map((link) => (
        <li key={link.slug}>
          <WikiLink slug={link.slug} href={link.href}>
            {link.text}
          </WikiLink>
          {link.note !== undefined && ` (${link.note})`}
          {link.notes.map((n) => (
            <Cite key={n} n={String(n)} />
          ))}
        </li>
      ))}
    </ul>
  );
}

// The relations the infobox doesn't show: the pages related both ways, what hurts it and what it hurts,
// what's found in a place.
export function SeeAlso({ meta }: { meta: PageMeta }) {
  if (meta.seeAlso.length === 0) return null;
  return (
    <section>
      <H2 id="see-also">See also</H2>
      <dl className="relations">
        {meta.seeAlso.map((relation) => (
          <div key={relation.relation}>
            <dt>{relation.label}</dt>
            <dd>
              <RelationLinks links={relation.links} />
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

// The pages whose text or relations link here, as Fandom's and Wikipedia's Special:WhatLinksHere lists them.
export function WhatLinksHere({ meta }: { meta: PageMeta }) {
  return (
    <section>
      <H2 id="what-links-here">What links here</H2>
      {meta.linksHere.length > 0 ? (
        <ul className="links-here">
          {meta.linksHere.map((page) => (
            <li key={page.slug}>
              <WikiLink slug={page.slug} href={`/wiki/${page.slug}`}>
                {page.title}
              </WikiLink>
            </li>
          ))}
        </ul>
      ) : (
        <p className="links-here-none">No other page links here yet.</p>
      )}
    </section>
  );
}
