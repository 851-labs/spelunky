// The wiki's page frame and a page's layout, Fandom's shape in the site's pixel look: breadcrumbs, the
// title and when it was verified, the infobox beside the lede, the body with its table of contents, the
// generated sections, and the navboxes of the page's categories. The license's notices are on every wiki
// page (docs/wiki-license.md, "Notice text for the templates"): the header line at the top and the footer.
//
// Everything has its size before it loads (the sprite, the clips' boxes) and nothing moves once it has: no
// layout shift.
import type { ReactNode } from "react";
import type { Crumb } from "../seo/jsonld";
import type { Navbox, Section } from "./build/compile";
import { CATEGORY_NAMES, hubPath } from "./build/categories.ts";
import { Cite, Clip, components, H2, H3, Sprite } from "./components";
import type { WikiComponents } from "./components";
import { trail } from "./crumbs";
import type { PageMeta, PageModule } from "./pages";
import { RelationLinks, SeeAlso, WhatLinksHere } from "./relations";

export function WikiFrame({ children }: { children: ReactNode }) {
  return (
    <div className="wiki">
      <a className="skip" href="#content">
        Skip to content
      </a>
      <header className="wiki-header">
        <a href="/wiki" className="wiki-home">
          <img src="/media/logo.png" width="160" height="36" alt="Spelunky" />
          <span className="wiki-badge">Fan wiki</span>
        </a>
        <p>Unofficial fan wiki. Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC.</p>
      </header>
      <main id="content">{children}</main>
      <footer className="wiki-footer">
        <p>
          Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC, and is used under the{" "}
          <a href="/LICENSE.txt">Spelunky User License v1.1b</a>. spelunky.ai is an unofficial fan project, not
          affiliated with or endorsed by Mossmouth. The game and its assets may not be sold.
        </p>
      </footer>
    </div>
  );
}

// The sections the layout adds after the body's, in order, when the page has them.
function generatedSections(meta: PageMeta): Section[] {
  return [
    ...(meta.questions.length > 0 ? [{ depth: 2 as const, id: "faq", title: "FAQ" }] : []),
    ...(meta.gallery.length > 0 ? [{ depth: 2 as const, id: "gallery", title: "Gallery" }] : []),
    ...(meta.seeAlso.length > 0 ? [{ depth: 2 as const, id: "see-also", title: "See also" }] : []),
    { depth: 2, id: "references", title: "References" },
    { depth: 2, id: "what-links-here", title: "What links here" },
  ];
}

export function WikiPage({ page }: { page: PageModule }) {
  const { meta, Body, Lede, Hatnote, Answers } = page;
  const clips = new Map(meta.clips.map((clip) => [clip.id, clip]));
  const sections = [...meta.sections, ...generatedSections(meta)];
  const body: WikiComponents = { ...components, Toc: () => <Contents sections={sections} /> };
  return (
    <>
      <article>
        <Breadcrumbs crumbs={trail(meta)} />
        <h1>{meta.title}</h1>
        <Verified verified={meta.verified} />
        {meta.spoiler && <p className="spoiler-note">This page describes a secret.</p>}
        {Hatnote && (
          <div className="hatnote">
            <Hatnote components={components} />
          </div>
        )}
        <div className="intro">
          <Infobox page={page} />
          <div className="lede">
            <Lede components={components} />
          </div>
        </div>
        <Body components={body} />
        {meta.questions.length > 0 && (
          <section>
            <H2 id="faq">FAQ</H2>
            <dl className="faq">
              {meta.questions.map((question, i) => {
                const Answer = Answers[i];
                return (
                  <div key={question}>
                    <dt>{question}</dt>
                    <dd>
                      <Answer components={components} />
                    </dd>
                  </div>
                );
              })}
            </dl>
          </section>
        )}
        {meta.gallery.length > 0 && (
          <section>
            <H2 id="gallery">Gallery</H2>
            {meta.gallery.map((id) => {
              const clip = clips.get(id)!;
              return <Clip key={id} {...clip} {...clip.files} />;
            })}
          </section>
        )}
        <SeeAlso meta={meta} />
        <section>
          <H2 id="references">References</H2>
          {meta.references.length > 0 && (
            <ol className="references">
              {meta.references.map((ref) => (
                <li key={ref.n} id={`cite-${ref.n}`}>
                  <Reference kind={ref.kind} target={ref.target} />
                </li>
              ))}
            </ol>
          )}
          <H3>Source code</H3>
          <ul className="sources">
            {meta.sources.map((source) => (
              <li key={source}>
                <code>{source}</code>
              </li>
            ))}
          </ul>
        </section>
        <WhatLinksHere meta={meta} />
      </article>
      <Navboxes navboxes={meta.navboxes} slug={meta.slug} />
    </>
  );
}

// The trail its BreadcrumbList JSON-LD has too (src/wiki/crumbs.ts).
export function Breadcrumbs({ crumbs }: { crumbs: Crumb[] }) {
  return (
    <nav className="breadcrumbs" aria-label="Breadcrumb">
      <ol>
        {crumbs.map((crumb, i) => (
          <li key={crumb.path}>
            {i === crumbs.length - 1 ? <span aria-current="page">{crumb.name}</span> : <a href={crumb.path}>{crumb.name}</a>}
          </li>
        ))}
      </ol>
    </nav>
  );
}

const MONTHS = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

// "Verified against v0.1.7" (docs/wiki-style.md, `verified`), under the title, where a reader judging the
// page looks first. The date's written out by hand: the same text on the server and in the browser.
function Verified({ verified }: { verified: PageMeta["verified"] }) {
  const [year, month, day] = verified.date.split("-").map(Number);
  return (
    <p className="verified">
      Verified against the spelunky.ai port{" "}
      <a href={`https://github.com/851-labs/spelunky/releases/tag/${verified.port}`}>{verified.port}</a> on{" "}
      <time dateTime={verified.date}>{`${day} ${MONTHS[month - 1]} ${year}`}</time>.
    </p>
  );
}

// The table of contents, numbered as Wikipedia's (1, 1.1...), each entry the section's stable anchor. A
// short page doesn't need one.
function Contents({ sections }: { sections: Section[] }) {
  if (sections.length < 3) return null;
  const tree: { section: Section; children: Section[] }[] = [];
  for (const section of sections) {
    const parent = tree.at(-1);
    if (section.depth === 3 && parent?.section.depth === 2) parent.children.push(section);
    else tree.push({ section, children: [] });
  }
  const entry = (section: Section) => <a href={`#${section.id}`}>{section.title}</a>;
  return (
    <nav className="toc" id="toc" aria-labelledby="toc-title">
      <p className="toc-title" id="toc-title">
        Contents
      </p>
      <ol>
        {tree.map(({ section, children }) => (
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
    </nav>
  );
}

// A citation as text, until the GML has a viewer (851-3382).
function Reference({ kind, target }: { kind: string; target: string }) {
  switch (kind) {
    case "gml":
      return <code>{target}</code>;
    case "replay":
      return (
        <>
          Replay <code>{target}</code>
        </>
      );
    case "clip":
      return (
        <>
          Clip <a href={`#clip-${target.replaceAll("/", "-")}`}>{target}</a>
        </>
      );
    case "quirk":
      return <>Quirk {target} (docs/quirks.md)</>;
    case "room":
      return (
        <>
          Room <code>{target}</code>, as the room editor placed it
        </>
      );
    default:
      return <code>{target}</code>;
  }
}

const notes = (ns: number[]) => ns.map((n) => <Cite key={n} n={String(n)} />);

// The largest the infobox shows a sprite: twice its @4x file (8 game pixels a pixel) when that fits, so a
// 16-pixel monster is 128 pixels wide; always a whole number of screen pixels a game pixel.
const INFOBOX_IMAGE_MAX = 224;

// The infobox, as Fandom's: the title, the sprite (animated, and still for reduced motion), the type's fields
// with their citations, the objects, then the relations it shows (schema.ts's INFOBOX_RELATIONS). A quirk
// page's quirk box is one too.
function Infobox({ page }: { page: PageModule }) {
  const { meta, Infobox: fragments } = page;
  if (!meta.image && meta.infobox.length === 0 && !meta.quirk) return null;
  const image = meta.image;
  const scale = image && image.width * 2 <= INFOBOX_IMAGE_MAX && image.height * 2 <= INFOBOX_IMAGE_MAX ? 2 : 1;
  return (
    <aside className="infobox" aria-labelledby="infobox-title">
      <p className="infobox-title" id="infobox-title">
        {meta.quirk ? "Quirk" : meta.title}
      </p>
      {image && (
        <div className="infobox-image">
          <Sprite
            name={image.sprite}
            alt={image.alt}
            width={image.width * scale}
            height={image.height * scale}
            src={image.src ?? undefined}
            still={image.anim ? (image.still ?? undefined) : undefined}
            loading="eager"
          />
        </div>
      )}
      <table>
        <tbody>
          {meta.infobox.map((row) => {
            const Fragment = fragments[row.field];
            return (
              <tr key={row.field}>
                <th scope="row">{row.label}</th>
                <td>
                  {Fragment ? (
                    <Fragment components={components} />
                  ) : row.value !== undefined ? (
                    <data value={row.value} data-cite={row.cite?.join(" ")}>
                      {row.text}
                    </data>
                  ) : (
                    row.text
                  )}
                  {notes(row.notes)}
                </td>
              </tr>
            );
          })}
          {meta.objects.length > 0 && (
            <tr>
              <th scope="row">{meta.objects.length === 1 ? "Object" : "Objects"}</th>
              <td>
                {meta.objects.map((object, i) => (
                  <span key={object}>
                    {i > 0 && ", "}
                    <code>{object}</code>
                  </span>
                ))}
              </td>
            </tr>
          )}
          {meta.quirk &&
            Object.entries(meta.quirk).map(([field, value]) => (
              <tr key={field}>
                <th scope="row">{field[0].toUpperCase() + field.slice(1).replaceAll("_", " ")}</th>
                <td>{value}</td>
              </tr>
            ))}
        </tbody>
        {meta.relations.length > 0 && (
          <tbody>
            <tr>
              <th colSpan={2} className="infobox-heading">
                Relations
              </th>
            </tr>
            {meta.relations.map((relation) => (
              <tr key={relation.relation}>
                <th scope="row">{relation.label}</th>
                <td>
                  <RelationLinks links={relation.links} />
                </td>
              </tr>
            ))}
          </tbody>
        )}
      </table>
    </aside>
  );
}

// A navbox per category the page is in (docs/wiki-style.md, "Links and navboxes"): the category's pages, by
// area where it has them, the page itself in bold.
function Navboxes({ navboxes, slug }: { navboxes: Navbox[]; slug: string }) {
  if (navboxes.length === 0) return null;
  const links = (group: Navbox["groups"][number]) => (
    <ul>
      {group.links.map((link) => (
        <li key={link.slug}>
          {link.slug === slug ? <strong aria-current="page">{link.title}</strong> : <a href={`/wiki/${link.slug}`}>{link.title}</a>}
        </li>
      ))}
    </ul>
  );
  return (
    <div className="navboxes">
      {navboxes.map((navbox) => {
        const id = `navbox-${navbox.category.replace("/", "-")}`;
        const name = CATEGORY_NAMES[navbox.category];
        const grouped = navbox.groups.some((group) => group.area !== null);
        return (
          <nav className="navbox" key={navbox.category} aria-labelledby={id}>
            <p className="navbox-title" id={id}>
              <a href={hubPath(navbox.category)}>{name}</a>
            </p>
            {grouped ? (
              <dl>
                {navbox.groups.map((group) => (
                  <div key={group.area ?? ""}>
                    <dt>{group.area ?? "Other"}</dt>
                    <dd>{links(group)}</dd>
                  </div>
                ))}
              </dl>
            ) : (
              links(navbox.groups[0])
            )}
          </nav>
        );
      })}
    </div>
  );
}
