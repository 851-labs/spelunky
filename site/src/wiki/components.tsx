// The components a page's MDX renders with (docs/wiki-style.md, "Body components"). compile.ts has already
// resolved what they show (a stat's text, a citation's number, a clip's files), so each only draws it.
import type { ComponentProps, ReactNode } from "react";
import { CardLink } from "./hover-card";
import { LiveClip } from "./live-clip";

// A number from stats.json: "1 heart". `value` is the bare number, `cite` the lines it came from.
function Stat({ text, value, cite }: { text: string; value: string; cite: string }) {
  return (
    <data value={value} data-cite={cite}>
      {text}
    </data>
  );
}

// A citation's note number, linking to its entry in References.
export function Cite({ n }: { n: string }) {
  return (
    <sup className="cite">
      <a href={`#cite-${n}`}>{`[${n}]`}</a>
    </sup>
  );
}

// A gml/ path or citation, linking to its lines in the source viewer when the media have them (compile.ts's
// sourceHref), and as text when they don't.
export function SourceLink({ href, children }: { href?: string; children: string }) {
  return href ? (
    <a href={href}>
      <code>{children}</code>
    </a>
  ) : (
    <code>{children}</code>
  );
}

export interface ClipProps {
  id: string;
  title: string;
  description: string;
  width: string | number;
  height: string | number;
  loop?: boolean;
  // Its files have the game's sound.
  audio?: boolean;
  mp4?: string;
  webm?: string;
  poster?: string;
  // Its replay, when the media have it (a scene's), and the replay's frame it starts on: then it can play in
  // the game too (live-clip.tsx).
  replay?: string;
  from?: string | number;
  // The game at /play/ seeks: the player has a slider (live-clip-player.tsx).
  seek?: boolean;
}

// A clip: the video, and its title and description from the spec. No license caption: the page's header
// line and footer carry the notice, and the footer the audio credit when a clip has sound
// (docs/wiki-license.md, Decisions). Played on request, not autoplayed: a page has several, and the prose
// beside each says what it shows. So it needn't be muted: the reader's press on play is what browsers ask
// before they play sound, and the native controls set its volume. One without sound is, so a phone doesn't
// pause the reader's music for it. A clip the media haven't got (a new spec, before scripts/wiki-media
// renders it) is its title and description in a box its size. It's at most 600 pixels tall, which the game's
// 4:3 clips never reach at the CSS's 640 wide: the settings program's tall window (416x600) shows its own size.
export function Clip({ id, title, description, width, height, loop, audio, mp4, webm, poster, replay, from, seek }: ClipProps) {
  const anchor = `clip-${id.replaceAll("/", "-")}`;
  const fit = Math.floor((600 * Number(width)) / Number(height));
  const video = mp4 && webm && (
    <video
      controls
      loop={loop}
      muted={!audio}
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
  );
  return (
    <figure className="clip" id={anchor} style={fit < 640 ? { maxWidth: fit } : undefined}>
      {video && replay ? (
        <LiveClip title={title} width={width} height={height} poster={poster} replay={replay} from={Number(from ?? 0)} seek={seek ?? false}>
          {video}
        </LiveClip>
      ) : (
        video || (
          <div className="clip-missing" style={{ aspectRatio: `${width} / ${height}` }}>
            Clip not rendered yet
          </div>
        )
      )}
      <figcaption>
        <strong>{title}.</strong> <span id={`${anchor}-description`}>{description}</span>
      </figcaption>
    </figure>
  );
}

export interface SpriteProps {
  name: string;
  alt: string;
  width: string | number;
  height: string | number;
  src?: string;
  // An animation's first frame, which it shows instead for `prefers-reduced-motion`.
  still?: string;
  loading?: "lazy" | "eager";
}

// A sprite at an integer scale, nearest neighbour (the CSS's image-rendering). Without the media, its alt
// text in a box.
export function Sprite({ alt, width, height, src, still, loading = "lazy" }: SpriteProps) {
  if (!src) {
    return (
      <span className="sprite-missing" role={alt ? "img" : undefined} aria-label={alt || undefined} aria-hidden={alt ? undefined : true} style={{ width: `${width}px`, height: `${height}px` }} />
    );
  }
  const img = <img className="sprite" src={src} width={width} height={height} alt={alt} loading={loading} />;
  if (!still) return img;
  return (
    <picture>
      <source media="(prefers-reduced-motion: reduce)" srcSet={still} />
      {img}
    </picture>
  );
}

// A section's heading, with a link to its anchor beside it (a page's stable links: /wiki/snake#behaviour)
// that shows on hover and focus. Outside the heading, so the heading's name is only its text.
function Heading({ level, id, children }: { level: 2 | 3; id?: string; children?: ReactNode }) {
  const H = level === 2 ? "h2" : "h3";
  return (
    <div className={`heading heading-${level}`}>
      <H id={id}>{children}</H>
      {id && (
        <a className="anchor" href={`#${id}`} aria-label="Link to this section" aria-describedby={id}>
          #
        </a>
      )}
    </div>
  );
}
export const H2 = (props: { id?: string; children?: ReactNode }) => <Heading level={2} {...props} />;
export const H3 = (props: { id?: string; children?: ReactNode }) => <Heading level={3} {...props} />;

// A [[link]]. A link to a planned page that isn't written yet is a red link, as on Fandom and Wikipedia:
// its text, marked, and no <a>, which would lead readers and crawlers to a 404 (docs/wiki-style.md, "Red
// links"). A link has a hover card (src/wiki/hover-card.tsx).
export function WikiLink({ slug, href, children }: { slug: string; href?: string; children?: ReactNode }) {
  if (!href) {
    return (
      <span className="wikilink-new" data-slug={slug} title="This page hasn't been written yet">
        {children}
      </span>
    );
  }
  return (
    <CardLink className="wikilink" href={href} slug={slug}>
      {children}
    </CardLink>
  );
}

// A ```gml fence: the lines, with the caption the license requires (docs/wiki-license.md, "GML excerpt
// caption"), its citation linking to them in the source viewer when the media have them (compile.ts).
function GmlExcerpt({ cite, code, href }: { cite: string; code: string; href?: string }) {
  return (
    <figure className="gml">
      <pre>
        <code>{code}</code>
      </pre>
      <figcaption>
        <SourceLink href={href}>{cite}</SourceLink>, from <code>spelunky.gmk</code>, Spelunky Classic 1.1's source, © 2008, 2009 Derek Yu
        and Mossmouth, LLC (<a href="/LICENSE.txt">Spelunky User License v1.1b</a>).
      </figcaption>
    </figure>
  );
}

// A body's table, in a box that scrolls it sideways on a narrow screen rather than widening the page.
function Table(props: ComponentProps<"table">) {
  return (
    <div className="table-wrap">
      <table {...props} />
    </div>
  );
}

// The table of contents compile.ts puts in a page's body: the layout's, which knows the page's sections.
const NoToc = () => null;

export const components = { Stat, Cite, Clip, Sprite, WikiLink, GmlExcerpt, h2: H2, h3: H3, table: Table, Toc: NoToc as () => ReactNode };
export type WikiComponents = typeof components;
