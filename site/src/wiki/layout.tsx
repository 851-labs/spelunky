// The wiki's page frame and a page's layout, Fandom's shape in the site's pixel look: breadcrumbs, the
// title, the infobox beside the lede, the body with its table of contents, the
// generated sections, and the navboxes of the page's categories; on a wide screen, the contents in a rail
// beside them (src/wiki/contents.tsx). The license's notices are on every wiki page (docs/wiki-license.md,
// "Notice text for the templates"): the footer, with the audio credit on a page whose clips have sound. The
// header has no copyright line (its Decisions, 5).
// The header has the search box (src/wiki/search-box.tsx), and links to pages have hover cards
// (src/wiki/hover-card.tsx).
//
// Everything has its size before it loads (the sprite, the clips' boxes) and nothing moves once it has: no
// layout shift.
import { Fragment } from "react";
import type { ReactNode } from "react";
import type { Crumb } from "../seo/jsonld";
import type { Navbox, Reference as ReferenceData, Section } from "./build/compile";
import { CATEGORY_NAMES, hubPath } from "./build/categories.ts";
import { Cite, Clip, components, H2, H3, SourceLink, Sprite } from "./components";
import type { WikiComponents } from "./components";
import { Contents, ContentsRail } from "./contents";
import { trail } from "./crumbs";
import { CardLink, HoverCards } from "./hover-card";
import { AUDIO_CREDIT, NOTICE } from "./notices.ts";
import type { PageMeta, PageModule } from "./pages";
import { RelationLinks, SeeAlso, WhatLinksHere } from "./relations";
import { SearchBox } from "./search-box";

export function WikiFrame({ children, audio = false }: { children: ReactNode; audio?: boolean }) {
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
        <SearchBox />
      </header>
      <main id="content">{children}</main>
      <HoverCards />
      <footer className="wiki-footer">
        <p>
          {NOTICE.before}
          <a href="/LICENSE.txt">{NOTICE.license}</a>
          {NOTICE.after}
        </p>
        {audio && <p>{AUDIO_CREDIT}</p>}
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
    <div className="wiki-page">
      <ContentsRail sections={sections} />
      <article>
        <Breadcrumbs crumbs={trail(meta)} />
        <h1>{meta.title}</h1>
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
                  <Reference reference={ref} />
                </li>
              ))}
            </ol>
          )}
          <H3>Source code</H3>
          <ul className="sources">
            {meta.sources.map((source) => (
              <li key={source.path}>
                <SourceLink href={source.href}>{source.path}</SourceLink>
              </li>
            ))}
          </ul>
        </section>
        <WhatLinksHere meta={meta} />
      </article>
      <Navboxes navboxes={meta.navboxes} slug={meta.slug} />
    </div>
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

const clipAnchor = (id: string) => `#clip-${id.replaceAll("/", "-")}`;

// A citation in References. Each links where it can be checked (docs/wiki-style.md, "Citing the code"): the
// lines in the source viewer, the replay on GitHub and the page's clips that play it, the clip, the quirk's row.
function Reference({ reference: { kind, target, href, clips, quirk } }: { reference: ReferenceData }) {
  switch (kind) {
    case "gml":
      return <SourceLink href={href}>{target}</SourceLink>;
    case "replay":
      return (
        <>
          Replay <SourceLink href={href}>{target}</SourceLink>
          {clips && (
            <>
              {clips.length === 1 ? ", which the clip " : ", which the clips "}
              {clips.map((clip, i) => (
                <Fragment key={clip.id}>
                  {i > 0 && (i === clips.length - 1 ? " and " : ", ")}
                  <a href={clipAnchor(clip.id)}>{clip.title}</a>
                </Fragment>
              ))}
              {clips.length === 1 ? " plays" : " play"}
            </>
          )}
        </>
      );
    case "clip":
      return (
        <>
          Clip <a href={clipAnchor(target)}>{target}</a>
        </>
      );
    case "quirk":
      return quirk ? <QuirkReference entry={target} quirk={quirk} /> : <>docs/quirks.md §{target}</>;
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

// A quirk citation: the rows it cites, by their openings, each linking to the quirk page that names it, or else
// to the row on GitHub, then the section, linking to the one row there when a page took its quote's link, else
// to the section. A citation of no row is the section's, by its title.
function QuirkReference({ entry, quirk }: { entry: string; quirk: NonNullable<ReferenceData["quirk"]> }) {
  const { rows } = quirk;
  if (rows.length === 0) return <a href={quirk.href}>{`docs/quirks.md §${entry}, ${quirk.section}`}</a>;
  const section = rows.length === 1 && rows[0].page ? rows[0].href : quirk.href;
  return (
    <>
      {rows.map((row, i) => (
        <Fragment key={row.name}>
          {i > 0 && " "}
          {row.page ? (
            <CardLink href={`/wiki/${row.page.slug}`} slug={row.page.slug}>
              <q>{row.name}</q>
            </CardLink>
          ) : (
            <a href={row.href}>
              <q>{row.name}</q>
            </a>
          )}
        </Fragment>
      ))}
      {", "}
      <a href={section}>{`docs/quirks.md §${entry}`}</a>
    </>
  );
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
          {meta.quirk && <QuirkRows quirk={meta.quirk} />}
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

// A quirk box's fields (docs/wiki-style.md, "Infobox fields"): how it's known, where the code is, what pins
// it (its replay on GitHub), and the docs/quirks.md rows the page is the readable version of, under their
// section's link, each linking to its row.
function QuirkRows({ quirk }: { quirk: NonNullable<PageMeta["quirk"]> }) {
  return (
    <>
      <tr>
        <th scope="row">Status</th>
        <td>{quirk.status}</td>
      </tr>
      <tr>
        <th scope="row">Where</th>
        <td>
          <SourceLink href={quirk.whereHref}>{quirk.where}</SourceLink>
        </td>
      </tr>
      <tr>
        <th scope="row">Pinned by</th>
        <td>
          <SourceLink href={quirk.pinnedHref}>{quirk.pinned_by}</SourceLink>
        </td>
      </tr>
      <tr>
        <th scope="row">{quirk.rows.length === 1 ? "Row" : "Rows"}</th>
        <td>
          <a href={quirk.href}>{`docs/quirks.md §${quirk.entry}, ${quirk.section}`}</a>
          {quirk.rows.map((row, i) => (
            <a key={row} className="infobox-row" href={quirk.rowHrefs[i]}>
              <q>{row}</q>
            </a>
          ))}
        </td>
      </tr>
    </>
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
          {link.slug === slug ? <strong aria-current="page">{link.title}</strong> : <CardLink href={`/wiki/${link.slug}`} slug={link.slug}>{link.title}</CardLink>}
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
