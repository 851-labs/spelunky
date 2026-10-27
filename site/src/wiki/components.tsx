// The components a page's MDX renders with (docs/wiki-style.md, "Body components"). compile.ts has already
// resolved what they show (a stat's text, a citation's number, a clip's files), so each only draws it.
//
// Some are stand-ins for what later tickets build: WikiLink is a plain link until 851-3334 (which makes a
// link to a missing page an error) and 851-3338 (hover cards), and the GML citations have no viewer until
// 851-3382.
import type { ReactNode } from "react";

// A number from stats.json: "1 heart". `value` is the bare number, `cite` the lines it came from.
function Stat({ text, value, cite }: { text: string; value: string; cite: string }) {
  return (
    <data value={value} data-cite={cite}>
      {text}
    </data>
  );
}

// A citation's note number, linking to its entry in References.
function Cite({ n }: { n: string }) {
  return (
    <sup className="cite">
      <a href={`#cite-${n}`}>{`[${n}]`}</a>
    </sup>
  );
}

export interface ClipProps {
  id: string;
  title: string;
  description: string;
  caption: string;
  width: string | number;
  height: string | number;
  loop?: boolean;
  mp4?: string;
  webm?: string;
  poster?: string;
}

// A clip: the video, its title and description from the spec, and the license's caption (docs/wiki-license.md,
// "Clip caption"). Played on request, not autoplayed: a page has several, and they're silent, so the prose
// beside each says what it shows. A clip the media haven't got yet (a new spec, before the next release
// renders it) is its title and description in a box its size.
export function Clip({ id, title, description, caption, width, height, loop, mp4, webm, poster }: ClipProps) {
  const anchor = `clip-${id.replaceAll("/", "-")}`;
  return (
    <figure className="clip" id={anchor}>
      {mp4 && webm ? (
        <video
          controls
          loop={loop}
          muted
          playsInline
          preload="none"
          poster={poster}
          width={width}
          height={height}
          aria-label={title}
          aria-describedby={`${anchor}-description`}
        >
          <source src={webm} type="video/webm" />
          <source src={mp4} type="video/mp4" />
        </video>
      ) : (
        <div className="clip-missing" style={{ aspectRatio: `${width} / ${height}` }}>
          Clip not rendered yet
        </div>
      )}
      <figcaption>
        <strong>{title}.</strong> <span id={`${anchor}-description`}>{description}</span>
        <small>{caption}</small>
      </figcaption>
    </figure>
  );
}

// A sprite at an integer scale, nearest neighbour (the CSS's image-rendering). Without the media, its alt
// text in a box.
export function Sprite({ alt, width, height, src }: { name: string; alt: string; width: string | number; height: string | number; src?: string }) {
  if (!src) {
    return (
      <span className="sprite-missing" role={alt ? "img" : undefined} aria-label={alt || undefined} aria-hidden={alt ? undefined : true} style={{ width: `${width}px`, height: `${height}px` }} />
    );
  }
  return <img className="sprite" src={src} width={width} height={height} alt={alt} loading="lazy" />;
}

// A [[link]]. A link to a page that doesn't exist yet is its text (851-3334 makes it an error).
function WikiLink({ slug, href, children }: { slug: string; href?: string; children?: ReactNode }) {
  if (!href) {
    return (
      <span className="wikilink-missing" data-slug={slug}>
        {children}
      </span>
    );
  }
  return (
    <a className="wikilink" href={href} data-slug={slug}>
      {children}
    </a>
  );
}

// A ```gml fence: the lines, with the caption the license requires (docs/wiki-license.md, "GML excerpt
// caption").
function GmlExcerpt({ cite, code }: { cite: string; code: string }) {
  return (
    <figure className="gml">
      <pre>
        <code>{code}</code>
      </pre>
      <figcaption>
        <code>{cite}</code>, from <code>spelunky.gmk</code>, Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu
        and Mossmouth, LLC (<a href="/LICENSE.txt">Spelunky User License v1.1b</a>).
      </figcaption>
    </figure>
  );
}

export const components = { Stat, Cite, Clip, Sprite, WikiLink, GmlExcerpt };
export type WikiComponents = typeof components;
