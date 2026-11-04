import { createFileRoute } from "@tanstack/react-router";
import css from "../landing.css?inline";
import { organization, videoGame, website } from "../seo/jsonld";
import { seo } from "../seo/seo";
import { SiteFrame, SiteLinks, SiteSearch } from "../site/frame";

// The landing page, in the site's frame (src/site/frame.tsx). Its CSS and the site's, inlined (landing.css imports
// site.css), like the hand-written page it replaced: no extra request before it paints.
export const Route = createFileRoute("/")({
  head: () => ({
    ...seo({
      title: "Spelunky Classic in your browser (unofficial port)",
      description:
        "An unofficial, tick-for-tick Rust port of Spelunky Classic 1.1 by Derek Yu. Play it in your browser.",
      path: "/",
      jsonLd: [website(), organization(), videoGame()],
    }),
    styles: [{ children: css }],
  }),
  component: Landing,
});

function Landing() {
  return (
    <SiteFrame
      section={null}
      search={<SiteSearch />}
      footer={<SiteLinks />}
    >
      {/* Play beside the gameplay on a wide screen, above it on a narrow one. */}
      <div className="hero">
        <div className="hero-text">
          <h1>Spelunky Classic 1.1, ported to Rust tick for tick, bugs included.</h1>
          <p className="notice">
            An unofficial fan port (a mod). Spelunky is Copyright &copy; 2008, 2009 Derek Yu and Mossmouth, LLC.
          </p>

          {/* Shown only on a device with only touch (landing.css): the game needs keys or a gamepad (851-3925). */}
          <p className="touch-note">
            This device seems to have no keyboard or gamepad. Spelunky Classic needs one: it has no touch controls.
          </p>
          <div className="play-row">
            <a className="play" href="/play/">
              Play in your browser
            </a>
            {/* 1.1's settings program, config.exe: the keys, the volumes and the window's size. */}
            <a className="settings" href="/play/?settings">
              Settings
            </a>
          </div>
          {/* The keys a new player needs in the first level (the game teaches them only in the Tutorial's signs);
              the wiki's Controls page has the rest. */}
          <p className="hint">
            Needs a keyboard or a gamepad: arrows move, <kbd>Z</kbd> jumps, <kbd>X</kbd> whips or uses what you
            hold, <kbd>A</kbd> throws a bomb, <kbd>S</kbd> a rope, <kbd>C</kbd> takes a bomb or rope out to hold,{" "}
            <kbd>Shift</kbd> runs, <kbd>P</kbd> pays in a shop and <kbd>Esc</kbd> pauses.{" "}
            <a href="/wiki/controls">All the controls</a> are in the wiki; Settings changes them. Saves stay in your
            browser.
          </p>
        </div>

        <div className="frame">
          <video
            src="media/gameplay.mp4"
            poster="media/gameplay.png"
            width="640"
            height="480"
            autoPlay
            loop
            muted
            playsInline
            aria-label="Gameplay: a run through the Mines"
          />
        </div>
      </div>

      <section>
        <h2>Desktop</h2>
        <p>
          Native builds are on <a href="https://github.com/851-labs/spelunky/releases/latest">GitHub Releases</a>.
        </p>
      </section>

      <section>
        <h2>About</h2>
        <p>
          Derek Yu's Spelunky Classic (2009), rebuilt from its GameMaker 8 source: the game's own code, transpiled
          to Rust, running on a reimplementation of the parts of GameMaker it uses. The goal is an exact replica,
          frame for frame, checked against the original. It runs natively and in the browser (WebGPU, or WebGL2
          where that's missing).
        </p>
        <p>
          Source: <a href="https://github.com/851-labs/spelunky">github.com/851-labs/spelunky</a>
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
          It's distributed under the <a href="/LICENSE.txt">Spelunky User License v1.1b</a>, and may not be
          sold.
        </p>
        <p>
          The site's pages use open-source libraries (React, Base UI, TanStack Router and others), under their own
          licenses: <a href="/third-party-licenses.txt">third-party licenses</a>.
        </p>
      </section>
    </SiteFrame>
  );
}
