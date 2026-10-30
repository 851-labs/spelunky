// A page's links to docs/quirks.md on GitHub (a quirk box's section and rows, a quirk citation's), out of its code
// (851-3744). Each names quirks.md's last change (repo.ts's filePins) and a row's line, so with them in the page's
// module, a change to quirks.md, which about a third of commits make, renamed every page that cites a quirk (~210)
// and the chunks that name them: ~220 of a build's ~285 scripts. So the module has a key for each link (`q0`, `q1`,
// ..., by first use on the page, which only the page's own citations change), and the page gets the links by key from its
// route's loader, at the pre-render, which puts them in the HTML and its hydration data, not in a chunk
// (src/routes/wiki/$slug.tsx, the wiki plugin's `virtual:wiki/quirk-links`). The Markdown twins, the hubs and the
// rest of the build have the links themselves (CompiledPage's `meta`).
import type { PageMeta } from "./build/compile";

// A link the page hasn't got by its key (the loader didn't run: a fixture's render), as unpinned as can be.
const QUIRKS_ON_MAIN = "https://github.com/851-labs/spelunky/blob/main/docs/quirks.md";

// Each quirks.md link a page's meta has, through `f`.
function mapQuirkLinks(meta: PageMeta, f: (href: string) => string): PageMeta {
  return {
    ...meta,
    quirk: meta.quirk && { ...meta.quirk, href: f(meta.quirk.href), rowHrefs: meta.quirk.rowHrefs.map(f) },
    references: meta.references.map((ref) =>
      ref.quirk ? { ...ref, quirk: { ...ref.quirk, href: f(ref.quirk.href), rows: ref.quirk.rows.map((row) => ({ ...row, href: f(row.href) })) } } : ref,
    ),
  };
}

// The meta for the page's module, its quirks.md links as keys, and the links by key.
export function keyQuirkLinks(meta: PageMeta): { meta: PageMeta; links: Record<string, string> } {
  const keys = new Map<string, string>();
  const keyed = mapQuirkLinks(meta, (href) => {
    if (!keys.has(href)) keys.set(href, `q${keys.size}`);
    return keys.get(href)!;
  });
  return { meta: keyed, links: Object.fromEntries([...keys].map(([href, key]) => [key, href])) };
}

const hasQuirkLinks = (meta: PageMeta) => meta.quirk !== null || meta.references.some((ref) => ref.quirk);

// The meta with its links back, by key; as it is for a page with none. A key the links haven't got (the loader ran
// in the browser, whose `virtual:wiki/quirk-links` is empty: a client-side navigation the site's plain links never
// make) links quirks.md on main, unpinned, and says so in the console rather than quietly.
export function resolveQuirkLinks(meta: PageMeta, links: Record<string, string> | undefined): PageMeta {
  if (!hasQuirkLinks(meta)) return meta;
  return mapQuirkLinks(meta, (key) => {
    if (links && Object.hasOwn(links, key)) return links[key];
    console.warn(`${meta.slug}: no link for ${key} (quirk-links.ts): linking docs/quirks.md on main`);
    return QUIRKS_ON_MAIN;
  });
}
