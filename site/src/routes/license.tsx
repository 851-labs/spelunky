import { createFileRoute } from "@tanstack/react-router";
import { license } from "virtual:site/license";
import type { Block } from "../build/license";
import { seo } from "../seo/seo";
import { SiteFrame, SiteLinks, SiteSearch } from "../site/frame";

// The Spelunky User License (851-4026), in the site's frame: the repo's LICENSE as the build reads it
// (src/build/license.ts), every word as the file has it, its headings as headings and its conditions as a list. It
// replaced the plain /LICENSE.txt, which the Worker redirects here (src/server.ts); the downloads and the wiki's
// media keep the plain file beside them, where the license asks for a copy. Its stylesheet is the root's to link
// (__root.tsx), as /download's is.
export const Route = createFileRoute("/license")({
  head: () =>
    seo({
      title: "Spelunky User License v1.1b - spelunky.ai",
      description:
        "The Spelunky User License v1.1b, which Spelunky Classic's game and source are under, and so this " +
        "unofficial fan port and its wiki: they may be modified and shared, never sold.",
      path: "/license",
    }),
  component: LicensePage,
});

function LicensePage() {
  return (
    <SiteFrame section={null} search={<SiteSearch />} footer={<SiteLinks />}>
      <h1>{license.title}</h1>
      {license.header.map((line, i) => (
        <p key={i} className="license-meta">
          {line}
        </p>
      ))}
      {/* What it means here, before the text: the conditions as spelunky.ai keeps them (docs/wiki-license.md). */}
      <aside className="license-here" aria-label="This port and the license">
        <p>
          spelunky.ai's port of Spelunky Classic and its wiki are a modified version of the game and its source (a
          mod), shared under this license. They're unofficial, made by fans, and not affiliated with or endorsed by
          Mossmouth; they're free, and may not be sold. The text below is the license's own, word for word: the{" "}
          <code>LICENSE</code> file every download has beside the game.
        </p>
      </aside>
      {license.sections.map((section) => (
        <section key={section.heading} aria-labelledby={anchor(section.heading)}>
          <h2 id={anchor(section.heading)}>{section.heading}</h2>
          {section.blocks.map((block, i) => (
            <LicenseBlock key={i} block={block} />
          ))}
        </section>
      ))}
      <section aria-labelledby="open-source">
        <h2 id="open-source">Open-source licenses</h2>
        <p>
          The site's pages use open-source libraries (React, Base UI, TanStack Router and others), under their own
          licenses: <a href="/third-party-licenses.txt">third-party licenses</a>. The game's, in the browser and in
          the desktop apps, are in its <code>THIRD-PARTY-LICENSES.txt</code>, beside it.
        </p>
      </section>
    </SiteFrame>
  );
}

// The conditions keep the file's own numbers, "(1)", as text: a list's markers would be another numbering.
function LicenseBlock({ block }: { block: Block }) {
  if (block.kind === "paragraph") return <p>{block.text}</p>;
  return (
    <ol className="conditions">
      {block.items.map((item) => (
        <li key={item.marker}>
          <span className="marker">{item.marker}</span>
          <span>{item.text}</span>
        </li>
      ))}
    </ol>
  );
}

// "3. Rights Granted" → "rights-granted", a section's fragment.
function anchor(heading: string): string {
  return heading
    .replace(/^\d+\.\s*/, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}
