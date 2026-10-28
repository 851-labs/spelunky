// The source viewer's pages (docs/wiki-media.md, "Source"): a file of the game's GML whole, with the license's
// notice above it (docs/wiki-license.md, "The source viewer"), line numbers that link to their lines, and the
// lines a link names highlighted (#L66, #L66-L71); and the index of every file. The highlighting is the build's
// (build/gml-highlight.ts): the browser gets markup and a few lines of script.
import type { Crumb } from "../seo/jsonld";
import { Breadcrumbs } from "./layout";
import { SOURCE_NOTICE, SUPER_SOUND_CREDIT } from "./notices.ts";
import { sourceSubject, sourceUrl } from "./source-url.ts";
import type { SourceFile, SourceIndex } from "./source";

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

export function SourceIndexView({ index }: { index: SourceIndex }) {
  const files = index.scripts.length + index.objects.reduce((sum, object) => sum + object.events.length, 0);
  return (
    <article className="source-page">
      <Breadcrumbs crumbs={[{ name: "Wiki", path: "/wiki" }, SOURCE]} />
      <h1>Source code</h1>
      <Notice path="" />
      <p className="source-about">
        {`Spelunky Classic 1.1's GML, ${files} files: ${count(index.scripts.length, "script")}, and the events of ` +
          `${count(index.objects.length, "object")}. The wiki's pages cite them by file and line.`}
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
    </article>
  );
}
