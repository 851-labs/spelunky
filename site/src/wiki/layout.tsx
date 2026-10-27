// The wiki's page frame and a page's layout. The license's notices are on every wiki page
// (docs/wiki-license.md, "Notice text for the templates"): the header line at the top and the footer.
//
// The parts the build generates are minimal here: the infobox (851-3335 lays it out, with the relations),
// the FAQ, the Gallery and the References. See also, What links here and the navboxes come with 851-3334
// and 851-3335.
import type { ReactNode } from "react";
import { Clip, components, Sprite } from "./components";
import type { PageModule } from "./pages";

export function WikiFrame({ children }: { children: ReactNode }) {
  return (
    <div className="wiki">
      <header className="wiki-header">
        <a href="/wiki" className="wiki-home">
          <img src="/media/logo.png" width="160" height="36" alt="Spelunky" />
        </a>
        <p>Unofficial fan wiki. Spelunky is © 2008, 2009 Derek Yu and Mossmouth, LLC.</p>
      </header>
      <main>{children}</main>
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

export function WikiPage({ page }: { page: PageModule }) {
  const { meta, Body, Lede, Hatnote, Answers } = page;
  const clips = new Map(meta.clips.map((clip) => [clip.id, clip]));
  return (
    <article>
      <h1>{meta.title}</h1>
      {meta.spoiler && <p className="spoiler-note">This page describes a secret.</p>}
      {Hatnote && (
        <div className="hatnote">
          <Hatnote components={components} />
        </div>
      )}
      <Infobox page={page} />
      <div className="lede">
        <Lede components={components} />
      </div>
      <Body components={components} />
      {meta.questions.length > 0 && (
        <section>
          <h2 id="faq">FAQ</h2>
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
          <h2 id="gallery">Gallery</h2>
          {meta.gallery.map((id) => {
            const clip = clips.get(id)!;
            return <Clip key={id} {...clip} {...clip.files} caption={meta.clipCaption} />;
          })}
        </section>
      )}
      <section>
        <h2 id="references">References</h2>
        {meta.references.length > 0 && (
          <ol className="references">
            {meta.references.map((ref) => (
              <li key={ref.n} id={`cite-${ref.n}`}>
                <Reference kind={ref.kind} target={ref.target} />
              </li>
            ))}
          </ol>
        )}
        <h3>Source code</h3>
        <ul className="sources">
          {meta.sources.map((source) => (
            <li key={source}>
              <code>{source}</code>
            </li>
          ))}
        </ul>
      </section>
      <p className="verified">
        Verified against the spelunky.ai port {meta.verified.port} on {meta.verified.date}.
      </p>
    </article>
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

// A stand-in for 851-3335's infobox: the image and the fields, in order.
function Infobox({ page }: { page: PageModule }) {
  const { meta, Infobox: fragments } = page;
  if (!meta.image && meta.infobox.length === 0 && !meta.quirk) return null;
  return (
    <aside className="infobox">
      <p className="infobox-title">{meta.title}</p>
      {meta.image && <Sprite name={meta.image.sprite} {...meta.image} src={meta.image.src ?? undefined} />}
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
                  {row.note !== undefined && <components.Cite n={String(row.note)} />}
                </td>
              </tr>
            );
          })}
          {meta.quirk &&
            Object.entries(meta.quirk).map(([field, value]) => (
              <tr key={field}>
                <th scope="row">{field[0].toUpperCase() + field.slice(1).replaceAll("_", " ")}</th>
                <td>{value}</td>
              </tr>
            ))}
        </tbody>
      </table>
    </aside>
  );
}
