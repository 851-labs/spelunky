// The source viewer's pages (docs/wiki-media.md, "Source"): a file of the game's GML whole, with the license's
// notice above it (docs/wiki-license.md, "The source viewer"), line numbers that link to their lines, and the
// lines a link names highlighted (#L66, #L66-L71); a room, its settings and what the room editor placed in it;
// and the index of every file and room. The highlighting is the build's (build/gml-highlight.ts): the browser
// gets markup and a few lines of script.
import type { Crumb } from "../seo/jsonld";
import { CardLink } from "./hover-card";
import { Breadcrumbs } from "./layout";
import { ROOM_NOTICE, SOURCE_NOTICE, SUPER_SOUND_CREDIT } from "./notices.ts";
import { roomUrl, sourceSubject, sourceUrl } from "./source-url.ts";
import type { RoomData, SourceFile, SourceIndex, SourceRoom } from "./source";

const SOURCE: Crumb = { name: "Source code", path: "/wiki/source" };

// The <code> the lines are in, which the script finds.
const CODE_ID = "source-code";

// Highlights the lines the URL's fragment names, as a stylesheet of their ids, kept up to date as it changes;
// and a line number shift-clicked names the lines from the one the selection started at (the line clicked, or a
// range's first as written) to it, as GitHub's do. Inline, after the code, so it runs as the page is parsed,
// before the browser has the page's scripts: a link to a range of a long file opens on it, not on the top and
// then jumps (one line is a :target, which the browser scrolls to itself). A stylesheet, not a class on the
// lines, so it never touches what React hydrates; and a string, the same in the server's HTML and the
// browser's render. A shift-click sets location.hash, as a click does, so :target follows it.
const HIGHLIGHT_SCRIPT =
  `(()=>{const code=document.getElementById(${JSON.stringify(CODE_ID)}),style=document.createElement("style");` +
  `let anchor=0;document.head.append(style);const mark=()=>{const m=/^#L(\\d+)(?:-L(\\d+))?$/.exec(location.hash);` +
  `anchor=m?+m[1]:0;let a=anchor,b=m?+(m[2]||m[1]):0;if(a>b)[a,b]=[b,a];b=Math.min(b,code.childElementCount);` +
  `const ids=[];for(let n=a;n>0&&n<=b;n++)ids.push("#L"+n);` +
  `style.textContent=ids.length?ids.join()+"{background:var(--line-hl)}":"";return a<b?a:0};` +
  `const first=mark();if(first)document.getElementById("L"+first).scrollIntoView({block:"center"});` +
  `addEventListener("hashchange",mark);code.addEventListener("click",(e)=>{const ln=e.target.closest("a.ln");` +
  `if(!ln||!e.shiftKey)return;e.preventDefault();const n=+ln.dataset.n;` +
  `location.hash=anchor&&anchor!==n?"#L"+anchor+"-L"+n:"#L"+n})})()`;

const count = (n: number, noun: string) => `${n} ${noun}${n === 1 ? "" : "s"}`;

export function sourceTrail(path: string): Crumb[] {
  return [{ name: "Wiki", path: "/wiki" }, SOURCE, { name: path, path: sourceUrl(path) }];
}

export function roomTrail(name: string): Crumb[] {
  return [{ name: "Wiki", path: "/wiki" }, SOURCE, { name: `rooms/${name}`, path: roomUrl(name) }];
}

function Notice({ path }: { path: string }) {
  return (
    <p className="source-notice">
      {SOURCE_NOTICE.before}
      <a href="/LICENSE.txt">{SOURCE_NOTICE.license}</a>
      {SOURCE_NOTICE.after}
      <code>{SOURCE_NOTICE.actionLines}</code>
      {SOURCE_NOTICE.end}
      {/^scripts\/SS_/.test(path) && ` ${SUPER_SOUND_CREDIT}`}
    </p>
  );
}

export function SourcePage({ file }: { file: SourceFile }) {
  const { path, lines, siblings } = file;
  const object = /^objects\/([^/]+)\//.exec(path)?.[1];
  return (
    <article className="source-page">
      <Breadcrumbs crumbs={sourceTrail(path)} />
      <h1>
        <code>{path}</code>
      </h1>
      <Notice path={path} />
      <p className="source-about">{`${sourceSubject(path).replace(/^t/, "T")}, ${count(lines.length, "line")} of GML.`}</p>
      {object && siblings.length > 1 && (
        <nav className="source-siblings" aria-label={`${object}'s events`}>
          <span>{object}'s events:</span>
          <ul>
            {siblings.map((sibling) => (
              <li key={sibling}>
                {`objects/${object}/${sibling}` === path ? (
                  <strong aria-current="page">{sibling}</strong>
                ) : (
                  <a href={sourceUrl(`objects/${object}/${sibling}`)}>{sibling}</a>
                )}
              </li>
            ))}
          </ul>
        </nav>
      )}
      <pre className="source-code">
        <code id={CODE_ID}>
          {lines.map((html, i) => {
            const n = i + 1;
            return (
              <span className="line" id={`L${n}`} key={n}>
                {/* The number is the CSS's, from data-n, so copying the code doesn't copy the numbers. */}
                <a className="ln" href={`#L${n}`} data-n={n} aria-label={`Line ${n}`} />
                {/* build/gml-highlight.ts's markup of the file's text: its <span>s and escaped text, nothing else. */}
                <span dangerouslySetInnerHTML={{ __html: html }} />
              </span>
            );
          })}
        </code>
      </pre>
      <script dangerouslySetInnerHTML={{ __html: HIGHLIGHT_SCRIPT }} />
    </article>
  );
}

// "(16, 32)".
const at = (x: number, y: number) => `(${x}, ${y})`;

// What a room's background slot shows, in words: "bgCave, tiled across and down".
function backgroundText(b: SourceRoom["backgrounds"][number]): string {
  const tiled = b.tile_h && b.tile_v ? "tiled across and down" : b.tile_h ? "tiled across" : b.tile_v ? "tiled down" : null;
  const parts = [
    b.background ?? "no background",
    b.foreground && "as a foreground",
    (b.x !== 0 || b.y !== 0) && `at ${at(b.x, b.y)}`,
    tiled,
    b.stretch && "stretched",
    (b.hspeed !== 0 || b.vspeed !== 0) && `moving ${b.hspeed} across and ${b.vspeed} down a step`,
    !b.visible && "hidden",
  ];
  return parts.filter(Boolean).join(", ");
}

// A view, in words: "320 × 240 at (0, 0), shown at 640 × 480, following oPlayer1 with borders of 128 and 96".
function viewText(v: SourceRoom["views"][number]): string {
  const parts = [
    `${v.view_w} × ${v.view_h} at ${at(v.view_x, v.view_y)}`,
    `shown at ${v.port_w} × ${v.port_h}${v.port_x !== 0 || v.port_y !== 0 ? ` at ${at(v.port_x, v.port_y)}` : ""}`,
    v.follow && `following ${v.follow} with borders of ${v.hborder} and ${v.vborder}`,
  ];
  return parts.filter(Boolean).join(", ");
}

// A room (/wiki/source/rooms/<name>): what the room editor shows of it, as text. Its settings; its instances by
// object, each object's row with its positions in creation order, an anchor of its name (#oSnake), and a link
// to its events in the index where it has any; its tiles; and its creation code and its instances', if any.
export function RoomPage({ data: { name, room, linked, pages, place } }: { data: RoomData }) {
  const objects = new Map<string, SourceRoom["instances"]>();
  for (const instance of room.instances) {
    const list = objects.get(instance.object);
    if (list) list.push(instance);
    else objects.set(instance.object, [instance]);
  }
  const names = [...objects.keys()].sort((a, b) => (a < b ? -1 : a > b ? 1 : 0));
  const withCode = room.instances.filter((instance) => instance.creation_code);
  return (
    <article className="source-page">
      <Breadcrumbs crumbs={roomTrail(name)} />
      <h1>
        <code>{room.name}</code>
      </h1>
      <p className="source-notice">
        {ROOM_NOTICE.before}
        <a href="/LICENSE.txt">{ROOM_NOTICE.license}</a>
        {ROOM_NOTICE.after}
      </p>
      <p className="source-about">
        {`The room ${room.name} as the room editor placed it: ${count(room.instances.length, "instance")} of ` +
          `${count(names.length, "object")}${room.tiles.length > 0 ? ` and ${count(room.tiles.length, "tile")}` : ""}.`}
      </p>
      {place && (
        <p className="source-about">
          {"The wiki's page on this room: "}
          <CardLink href={`/wiki/${place.slug}`} slug={place.slug}>
            {place.title}
          </CardLink>
          .
        </p>
      )}
      <h2 id="settings">Settings</h2>
      <table className="room-settings">
        <tbody>
          {room.caption && (
            <tr>
              <th scope="row">Caption</th>
              <td>{room.caption}</td>
            </tr>
          )}
          <tr>
            <th scope="row">Size</th>
            <td>{`${room.width} × ${room.height} pixels`}</td>
          </tr>
          <tr>
            <th scope="row">Speed</th>
            <td>{`${room.speed} steps a second`}</td>
          </tr>
          <tr>
            <th scope="row">Persistent</th>
            <td>{room.persistent ? "Yes" : "No"}</td>
          </tr>
          <tr>
            <th scope="row">Background colour</th>
            <td>
              <code>{room.background_color}</code>
              {room.draw_background_color ? ", drawn" : ", not drawn"}
            </td>
          </tr>
          <tr>
            <th scope="row">Backgrounds</th>
            <td>
              {room.backgrounds.length === 0
                ? "None"
                : room.backgrounds.map((b) => (
                    <div key={b.slot}>{`${room.backgrounds.length > 1 || b.slot !== 0 ? `${b.slot}: ` : ""}${backgroundText(b)}`}</div>
                  ))}
            </td>
          </tr>
          <tr>
            <th scope="row">Views</th>
            <td>
              {!room.views_enabled
                ? "Off"
                : room.views.length === 0
                  ? "On, none visible"
                  : room.views.map((v) => <div key={v.slot}>{`View ${v.slot}: ${viewText(v)}`}</div>)}
            </td>
          </tr>
          <tr>
            <th scope="row">Creation code</th>
            <td>{room.creation_code ? <a href="#creation-code">Below</a> : "None"}</td>
          </tr>
        </tbody>
      </table>
      <h2 id="instances">Instances</h2>
      <table className="room-instances">
        <thead>
          <tr>
            <th scope="col">Object</th>
            <th scope="col">Instances</th>
            <th scope="col">Positions (x, y)</th>
          </tr>
        </thead>
        <tbody>
          {names.map((name) => {
            const instances = objects.get(name)!;
            return (
              <tr key={name} id={name}>
                <th scope="row">
                  {linked.includes(name) ? (
                    <a href={`/wiki/source#${name}`}>
                      <code>{name}</code>
                    </a>
                  ) : (
                    <code>{name}</code>
                  )}
                  {Object.hasOwn(pages, name) && (
                    <CardLink className="room-page" href={`/wiki/${pages[name].slug}`} slug={pages[name].slug}>
                      {pages[name].title}
                    </CardLink>
                  )}
                </th>
                <td>{instances.length}</td>
                <td>
                  <ul className="room-positions">
                    {instances.map((i) => (
                      <li key={i.id} title={`Instance ${i.id}`}>
                        {at(i.x, i.y)}
                      </li>
                    ))}
                  </ul>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      {room.tiles.length > 0 && (
        <>
          <h2 id="tiles">Tiles</h2>
          <table className="room-instances">
            <thead>
              <tr>
                <th scope="col">Background</th>
                <th scope="col">Position</th>
                <th scope="col">From the background</th>
                <th scope="col">Size</th>
                <th scope="col">Depth</th>
              </tr>
            </thead>
            <tbody>
              {room.tiles.map((t) => (
                <tr key={t.id}>
                  <th scope="row">
                    <code>{t.background}</code>
                  </th>
                  <td>{at(t.x, t.y)}</td>
                  <td>{at(t.bg_x, t.bg_y)}</td>
                  <td>{`${t.width} × ${t.height}`}</td>
                  <td>{t.depth}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      )}
      {room.creation_code && (
        <>
          <h2 id="creation-code">Creation code</h2>
          <pre className="source-code room-code">
            <code>{room.creation_code}</code>
          </pre>
        </>
      )}
      {withCode.length > 0 && (
        <>
          <h2 id="instance-creation-code">Instances' creation code</h2>
          {withCode.map((i) => (
            <section key={i.id}>
              <h3>{`Instance ${i.id} of ${i.object} at ${at(i.x, i.y)}`}</h3>
              <pre className="source-code room-code">
                <code>{i.creation_code}</code>
              </pre>
            </section>
          ))}
        </>
      )}
    </article>
  );
}

export function SourceIndexView({ index }: { index: SourceIndex }) {
  const files = index.scripts.length + index.objects.reduce((sum, object) => sum + object.events.length, 0);
  return (
    <article className="source-page">
      <Breadcrumbs crumbs={[{ name: "Wiki", path: "/wiki" }, SOURCE]} />
      <h1>Source code</h1>
      <Notice path="" />
      <p className="source-about">
        {`Spelunky Classic 1.1's GML, ${files} files: ${count(index.scripts.length, "script")}, and the events of ` +
          `${count(index.objects.length, "object")}; and its ${count(index.rooms.length, "room")}. The wiki's pages ` +
          "cite the GML by file and line, and the rooms by name."}
      </p>
      <h2 id="scripts">Scripts</h2>
      <ul className="source-list">
        {index.scripts.map((file) => (
          <li key={file}>
            <a href={sourceUrl(`scripts/${file}`)}>{file.replace(/\.gml$/, "")}</a>
          </li>
        ))}
      </ul>
      <h2 id="objects">Objects</h2>
      <dl className="source-objects">
        {index.objects.map(({ name, events }) => (
          <div key={name}>
            <dt id={name}>{name}</dt>
            <dd>
              <ul>
                {events.map((file) => (
                  <li key={file}>
                    <a href={sourceUrl(`objects/${name}/${file}`)}>{file.replace(/\.gml$/, "")}</a>
                  </li>
                ))}
              </ul>
            </dd>
          </div>
        ))}
      </dl>
      {index.rooms.length > 0 && (
        <>
          <h2 id="rooms">Rooms</h2>
          <ul className="source-list">
            {index.rooms.map((name) => (
              <li key={name}>
                <a href={roomUrl(name)}>{name}</a>
              </li>
            ))}
          </ul>
        </>
      )}
    </article>
  );
}
