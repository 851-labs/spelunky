// A clip in the game itself: a clip whose replay the media have (every clip's but the settings program's) has a
// "Play live" button under its video, which swaps the video for the web build at /play/ replaying the clip from
// its first frame, with controls to pause it and to take over from the replay (live-clip-player.tsx).
//
// Nothing of the game loads until the click: the page has only this button. The player is a chunk of its own,
// and the game is a same-origin iframe of /play/?replay=URL&from=F&embed (crates/app's window::Embed), made
// then, so it's /play/'s own page and assets, cached for good, and none of them is the page's JS.
//
// Phones play it too, when the game at /play/ can (`phones`, build.sh's WIKI_LIVE_PHONES: it starts its sound at
// a tap where the browser held it, as Safari does, which every browser on iOS is). Before that, wiki.css hides
// the button where the pointer is coarse. A phone gets the clip to watch, pause and scrub, not Take control: the
// game needs a keyboard, and on-screen buttons for its eight keys would cover most of a phone-sized clip. The
// video stays where the game can't run well: a browser without WebAssembly or WebGL2, a reader saving data
// (the game is megabytes), and a device the player finds can't keep up, or can't get to the clip's first frame
// in time. And where the game plays the clip unlike its video: /play/ is a release, the media are rendered from the
// main branch, and a fix there since the release can change the clip (`state`, build.sh's WIKI_LIVE_STATE: the game
// reports its state at the clip's first frame, and WIKI_LIVE_STATES: it runs the clip through first, and reports its
// states at each of its frames). That won't change on another try, so Play live goes.
import { Button } from "@base-ui/react/button";
import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import type { LiveClipPlayer, Slow } from "./live-clip-player";
import { loadOnce, never } from "./load-once";

// The Worker never shows it, so its bundle needn't carry it (load-once.ts).
const loadPlayer = loadOnce(() =>
  import.meta.env.SSR ? never() : import("./live-clip-player").then((module) => module.LiveClipPlayer),
);

export interface LiveClipProps {
  title: string;
  width: string | number;
  height: string | number;
  poster?: string;
  // The replay's URL, and its frame the clip starts on.
  replay: string;
  from: number;
  // Whether the game seeks (live-clip-player.tsx).
  seek: boolean;
  // Whether phones can play it.
  phones: boolean;
  // Whether the game reports its start-up run (live-clip-player.tsx).
  progress: boolean;
  // The game's state at the clip's first frame, which the game's must be (live-clip-player.tsx), or null when the
  // game doesn't report it.
  state: string | null;
  // The game's states at each of the clip's frames, which the game's must be, or null when the game doesn't run the
  // clip through and report them.
  states: string | null;
  // The video.
  children: ReactNode;
}

export function LiveClip({ title, width, height, poster, replay, from, seek, phones, progress, state: firstState, states, children }: LiveClipProps) {
  // The player, once its chunk is in: until then, and if it can't load, the video stays.
  const [Player, setPlayer] = useState<typeof LiveClipPlayer | null>(null);
  const [state, setState] = useState<"idle" | "loading" | "failed" | Slow | "unlike">("idle");
  // Pre-rendered with the button, which goes once the browser turns out not to run the game well.
  const [runs, setRuns] = useState(true);
  useEffect(() => setRuns(runsTheGame()), []);
  if (Player) {
    return (
      <Player
        title={title}
        width={width}
        height={height}
        poster={poster}
        replay={replay}
        from={from}
        seek={seek}
        progress={progress}
        state={firstState}
        states={states}
        onClose={() => setPlayer(null)}
        onSlow={(slow) => {
          setPlayer(null);
          setState(slow);
        }}
        onUnlike={() => {
          setPlayer(null);
          setState("unlike");
        }}
      />
    );
  }
  const start = () => {
    setState("loading");
    loadPlayer().then(
      (player) => {
        setState("idle");
        // A component is a function, which setState would call.
        setPlayer(() => player);
      },
      () => setState("failed"),
    );
  };
  return (
    <>
      {children}
      {runs && (
        <div className="clip-live" data-phones={phones ? "" : undefined}>
          {state !== "unlike" && (
            <Button className="clip-button" onClick={start} disabled={state === "loading"} focusableWhenDisabled>
              Play live
            </Button>
          )}
          <span className="clip-live-note" role="status">
            {state === "unlike" ? (
              "This release of the game plays this clip differently from its video, so here's the video."
            ) : state === "failed" ? (
              "Couldn't load the game's controls. Try again?"
            ) : state === "slow" ? (
              // Still offered: it may have been a busy moment.
              "This device couldn't run the game at full speed, so here's the video."
            ) : state === "late" ? (
              "This device couldn't get the game to the clip in time, so here's the video."
            ) : (
              // Which one shows is wiki.css's: a touch screen has no keys to take over with.
              <>
                <span className="clip-live-keys">Replays it in the game, where you can take over.</span>
                <span className="clip-live-touch">Replays it in the game, to pause and scrub.</span>
              </>
            )}
          </span>
        </div>
      )}
    </>
  );
}

// Whether this browser can run the game well enough to offer it: WebAssembly, WebGL2 (the game's fallback from
// WebGPU, which every browser with WebGPU has too), and no Save-Data request from the reader.
function runsTheGame(): boolean {
  const connection = (navigator as Navigator & { connection?: { saveData?: boolean } }).connection;
  return typeof WebAssembly === "object" && typeof WebGL2RenderingContext === "function" && !connection?.saveData;
}
