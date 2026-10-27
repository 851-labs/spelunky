import { createFileRoute } from "@tanstack/react-router";
import css from "../landing.css?raw";
import { organization, videoGame, website } from "../seo/jsonld";
import { seo } from "../seo/seo";

// The landing page. Inlined CSS, like the hand-written page it replaced: no extra request before it paints.
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
    <main>
      <h1>
        <img className="logo" src="media/logo.png" width="160" height="36" alt="Spelunky" />
      </h1>
      <p className="tagline">Spelunky Classic 1.1, ported to Rust tick for tick, bugs included.</p>
      <p className="notice">
        An unofficial fan port (a mod). Spelunky is Copyright &copy; 2008, 2009 Derek Yu and Mossmouth, LLC.
      </p>

      <a className="play" href="/play/">
        Play in your browser
      </a>
      <p className="hint">
        Needs a keyboard or a gamepad: arrows move, <kbd>Z</kbd> jumps, <kbd>X</kbd> whips. Saves stay in your
        browser.
      </p>

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
      </section>

      <footer>
        spelunky.ai &middot; <a href="https://github.com/851-labs/spelunky">GitHub</a>
      </footer>
    </main>
  );
}
