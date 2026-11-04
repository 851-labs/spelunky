import { createFileRoute } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { release } from "virtual:site/release";
import { seo } from "../seo/seo";
import { browserSystem, detect, downloads, pick, size } from "../site/download";
import type { Download, Os, System } from "../site/download";
import { SiteFrame, SiteLinks, SiteSearch } from "../site/frame";

// The native apps' page (851-4013): the release the site puts live, each system's files from its update manifest
// (src/build/release.ts), with their sizes, so the page changes with web-release.txt and lists what
// the apps update to. The files stay on GitHub's releases; the page links them. Its stylesheet is the root's to link
// (__root.tsx), as the 404 page's is: inlined, it would be in the route tree, every page's entry chunk.
export const Route = createFileRoute("/download")({
  head: () =>
    seo({
      title: "Download Spelunky Classic for macOS, Windows and Linux (unofficial port)",
      description:
        "The unofficial Rust port of Spelunky Classic 1.1 as a desktop app: macOS (universal, notarized), Windows " +
        "and Linux, x64 and ARM64. Free, and it updates itself.",
      path: "/download",
    }),
  component: DownloadPage,
});

const RELEASES = "https://github.com/851-labs/spelunky/releases";
// The build's: its manifest's files, by system.
const LIST = release ? downloads(release) : null;

function DownloadPage() {
  // The reader's system, once the page is in their browser: the pre-rendered page has none, and highlights none.
  const [yours, setYours] = useState<System | null>(null);
  useEffect(() => {
    let live = true;
    browserSystem(navigator).then((browser) => live && setYours(detect(browser)));
    return () => {
      live = false;
    };
  }, []);
  const list = LIST;
  const mine = list?.systems.find((system) => system.os === yours?.os);
  const first = mine && pick(mine.downloads, yours?.arch ?? null);
  return (
    <SiteFrame section="download" search={<SiteSearch />} footer={<SiteLinks />}>
      <h1>Download Spelunky Classic</h1>
      {release && list ? (
        <>
          <p className="lede">
            Version {release.version} of the port, as a desktop app for {names(list.systems.map((s) => s.name))}. It's free, and it
            updates itself: it checks once a day, between runs, and asks first. Or{" "}
            <a href="/play/">play in your browser</a>.
          </p>
          {mine && first && (
            <div className="yours">
              <a className="download-button" href={first.url}>
                Download for {mine.name}
              </a>
              <p className="hint">
                {first.label} {first.kind}, {size(first.size)}. The others are below.
              </p>
            </div>
          )}
          <div className="systems">
            {list.systems.map((system) => (
              <SystemCard
                key={system.os}
                {...system}
                yours={system.os === mine?.os}
                first={system.os === mine?.os ? first : undefined}
              />
            ))}
          </div>
          {list.other.length > 0 && (
            <section>
              <h2>Other builds</h2>
              <Files downloads={list.other} />
            </section>
          )}
          <section>
            <h2>Other files and versions</h2>
            <p>
              The macOS app as a disk image (<code>.dmg</code>), the Linux builds as tarballs, and every earlier
              version are on GitHub: <a href={release.notes}>{release.version}'s release notes and files</a>, and{" "}
              <a href={RELEASES}>all releases</a>.
            </p>
            <p>
              To check a download, each file's SHA-256 is in the release's <code>SHA256SUMS</code>, with its notes.
            </p>
          </section>
        </>
      ) : (
        // A build without the release's manifest (`vite dev`, a release without one): GitHub has the files.
        <p className="lede">
          The desktop apps for macOS, Windows and Linux are on <a href={`${RELEASES}/latest`}>GitHub's releases</a>.
          Or <a href="/play/">play in your browser</a>.
        </p>
      )}
      <section>
        <h2>Saves</h2>
        <p>
          The apps keep your stats, highscores, settings and levels apart from the app, for each user, so an update
          or a new download keeps them: in <code>~/Library/Application Support/Spelunky Classic</code> on macOS,{" "}
          <code>%APPDATA%\Spelunky Classic</code> on Windows, and <code>~/.local/share/spelunky-classic</code> on
          Linux. A browser's saves stay in the browser.
        </p>
      </section>
      <section>
        <h2>Credits and license</h2>
        <p>
          <a href="https://spelunkyworld.com">Spelunky</a> is by Derek Yu, Copyright &copy; 2008, 2009 Derek Yu
          and Mossmouth, LLC. This port is a modified version of the Spelunky source, made by fans: it isn't
          official, and isn't affiliated with Mossmouth.
        </p>
        <p>
          It's distributed under the <a href="/license">Spelunky User License v1.1b</a>, and may not be sold.
          Each download has the license, a notice and its third-party licenses beside the game. Source:{" "}
          <a href="https://github.com/851-labs/spelunky">github.com/851-labs/spelunky</a>.
        </p>
      </section>
    </SiteFrame>
  );
}

// "macOS, Windows and Linux".
function names(list: string[]): string {
  return list.length > 1 ? `${list.slice(0, -1).join(", ")} and ${list[list.length - 1]}` : (list[0] ?? "");
}

function SystemCard({
  os,
  name,
  needs,
  downloads,
  yours,
  first,
}: {
  os: Os;
  name: string;
  needs: string;
  downloads: Download[];
  yours: boolean;
  first: Download | undefined;
}) {
  return (
    <section className="system" data-yours={yours || undefined} aria-labelledby={`system-${os}`}>
      <h2 id={`system-${os}`}>
        {name}
        {yours && <span className="badge">Your system</span>}
      </h2>
      <p className="needs">{needs}</p>
      <Files downloads={downloads} first={first} />
      <h3>Installing</h3>
      <Install os={os} file={(first ?? downloads[0])?.file ?? ""} />
    </section>
  );
}

function Files({ downloads, first }: { downloads: Download[]; first?: Download }) {
  return (
    <ul className="files">
      {downloads.map((download) => (
        <li key={download.platform} data-first={download === first || undefined}>
          <a className="file" href={download.url}>
            <span className="file-label">{download.label}</span>
            <span className="file-meta">
              {download.kind}, {size(download.size)}
            </span>
          </a>
        </li>
      ))}
    </ul>
  );
}

// What to do with the file once it's downloaded. The macOS app is signed and notarized, the others aren't signed
// (docs/releasing.md).
function Install({ os, file }: { os: Os; file: string }) {
  switch (os) {
    case "macos":
      return (
        <ol>
          <li>Open the zip (Safari opens it itself) to get Spelunky Classic.</li>
          <li>Move it to Applications, and open it. It's signed and notarized by Apple, so it opens as any app does.</li>
        </ol>
      );
    case "windows":
      return (
        <ol>
          <li>
            Extract the zip (right-click it, <b>Extract All</b>) to a folder of your own, such as your Documents:
            not Program Files, where the app can't update itself.
          </li>
          <li>
            Run <code>spelunky.exe</code> in it. The app isn't signed, so SmartScreen may say "Windows protected your
            PC": choose <b>More info</b>, then <b>Run anyway</b>.
          </li>
        </ol>
      );
    case "linux":
      return (
        <ol>
          <li>
            Make it executable: <code>chmod +x {file}</code>, or tick "Allow executing file as program" in its
            properties.
          </li>
          <li>
            Run it. On a system without FUSE, run it with <code>--appimage-extract-and-run</code>.
          </li>
        </ol>
      );
  }
}
