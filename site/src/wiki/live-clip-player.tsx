// The game in a clip's box (live-clip.tsx): /play/ in a same-origin iframe, replaying the clip, and Base UI's
// slider and toolbar of controls under it. The page drives the game with postMessage, and the game tells it
// where its replay is (crates/app's web::listen_to_embedder and tell_embedder):
//
//   page to game  "spelunky:pause", "spelunky:play", "spelunky:take-control", { spelunky: "seek", frame }
//   game to page  { spelunky: "playback", ready, paused, live, ended, frame, frames }, from its start, before
//                 its downloads
//                 { spelunky: "error", error }
//
// The game runs the replay's frames before the clip's unshown, as fast as they go, and says it's ready on the
// clip's first, so the poster stays over it until then. At the replay's end it holds the last frame, from which
// the reader can take over too. A game that seeks (`seek`, build.sh's WIKI_LIVE_SEEK, from /play/'s page) keeps
// savestates as it plays: the slider seeks in the clip, handing the game back to the replay if the reader has
// it, and Restart is a seek to its first frame. Without, it has no frame or frames, and Restarting is loading
// the iframe again: /play/'s files are cached for good.
import { Slider } from "@base-ui/react/slider";
import { Toolbar } from "@base-ui/react/toolbar";
import { useEffect, useRef, useState } from "react";

interface Playback {
  ready: boolean;
  paused: boolean;
  live: boolean;
  ended: boolean;
  // The clip's frame showing (or being sought), and its frames: a game that seeks says.
  frame?: number;
  frames?: number;
}

type Engine =
  // Its page is loading, or the game is starting: nothing heard yet.
  | { kind: "loading" }
  | { kind: "playback"; playback: Playback }
  | { kind: "error"; error: string };

// The game's 30 frames a second.
const FPS = 30;

export interface LiveClipPlayerProps {
  title: string;
  width: string | number;
  height: string | number;
  poster?: string;
  replay: string;
  from: number;
  // The game seeks: a slider, and an instant Restart.
  seek: boolean;
  // Back to the video.
  onClose: () => void;
}

export function LiveClipPlayer({ title, width, height, poster, replay, from, seek, onClose }: LiveClipPlayerProps) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [engine, setEngine] = useState<Engine>({ kind: "loading" });
  // A new iframe each time, for Restart without seeking.
  const [run, setRun] = useState(0);
  // Where the reader put the slider, until the game reports that frame (or a moment has passed, whatever it
  // reports): its reports from before it had the seek would pull the thumb back.
  const [scrub, setScrub] = useState<number | null>(null);
  const scrubTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(scrubTimer.current), []);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      // Only this box's game: a page can have several.
      if (event.origin !== location.origin || event.source !== frame.current?.contentWindow) return;
      const data = event.data;
      if (data?.spelunky === "playback") {
        setEngine({ kind: "playback", playback: data });
        setScrub((at) => (at === data.frame ? null : at));
      } else if (data?.spelunky === "error") setEngine({ kind: "error", error: String(data.error) });
    };
    addEventListener("message", listener);
    return () => removeEventListener("message", listener);
  }, []);

  // Whole, so that site/test-bundle.mjs can find them.
  const send = (message: "spelunky:pause" | "spelunky:play" | "spelunky:take-control" | { spelunky: "seek"; frame: number }) =>
    frame.current?.contentWindow?.postMessage(message, location.origin);
  const playback = engine.kind === "playback" ? engine.playback : null;
  const ready = playback?.ready ?? false;
  const live = playback?.live ?? false;
  const ended = playback?.ended ?? false;
  const paused = playback?.paused ?? false;
  // The slider, once the game says where it is.
  const frames = seek && playback?.frames !== undefined && playback.frames > 0 ? playback.frames : null;
  const seekTo = (to: number) => {
    setScrub(to);
    clearTimeout(scrubTimer.current);
    scrubTimer.current = setTimeout(() => setScrub(null), 500);
    send({ spelunky: "seek", frame: to });
  };
  const restart = () => {
    if (frames !== null) {
      seekTo(0);
      send("spelunky:play");
      return;
    }
    setEngine({ kind: "loading" });
    setRun((n) => n + 1);
  };
  const takeControl = () => {
    send("spelunky:take-control");
    // The game focuses its canvas too; this is the frame's half, while the click is still a user gesture.
    frame.current?.focus();
  };
  const src = `/play/?replay=${encodeURIComponent(replay)}&from=${from}&embed`;
  // The poster covers the game until the clip's first frame.
  const covered = engine.kind === "loading" || (playback !== null && !playback.ready);
  // The reader's game runs on past the clip's end: the thumb stays there.
  const at = frames === null ? 0 : Math.min(scrub ?? playback?.frame ?? 0, frames - 1);
  return (
    <div className="clip-player">
      <div className="clip-engine" style={{ aspectRatio: `${width} / ${height}` }}>
        <iframe
          key={run}
          ref={frame}
          src={src}
          title={`${title}, in the game`}
          allow="autoplay; fullscreen; gamepad"
        />
        {covered && (
          <div className="clip-engine-cover" style={poster ? { backgroundImage: `url("${poster}")` } : undefined}>
            <span>Starting the game…</span>
          </div>
        )}
      </div>
      {frames !== null && (
        <div className="clip-seek">
          <Slider.Root
            className="clip-slider"
            value={at}
            min={0}
            max={frames - 1}
            disabled={!ready}
            onValueChange={(value) => seekTo(value)}
          >
            <Slider.Control className="clip-slider-control">
              <Slider.Track className="clip-slider-track">
                <Slider.Indicator className="clip-slider-indicator" />
                <Slider.Thumb
                  className="clip-slider-thumb"
                  getAriaLabel={() => `${title}: where in the clip`}
                  getAriaValueText={(_, value) => `${seconds(value)} of ${seconds(frames)} seconds`}
                />
              </Slider.Track>
            </Slider.Control>
          </Slider.Root>
          <span className="clip-time" aria-hidden="true">
            {seconds(at)} / {seconds(frames)}
          </span>
        </div>
      )}
      <Toolbar.Root className="clip-controls" aria-label={`${title}: the game's controls`}>
        <Toolbar.Button className="clip-button" disabled={!ready || live || ended} onClick={() => send(paused ? "spelunky:play" : "spelunky:pause")}>
          {paused && !ended ? "Play" : "Pause"}
        </Toolbar.Button>
        <Toolbar.Button className="clip-button" disabled={!ready || live} onClick={takeControl}>
          Take control
        </Toolbar.Button>
        <Toolbar.Separator className="clip-controls-gap" />
        <Toolbar.Button className="clip-button clip-button-quiet" disabled={frames !== null && !ready} onClick={restart}>
          Restart
        </Toolbar.Button>
        <Toolbar.Button className="clip-button clip-button-quiet" onClick={onClose}>
          Back to the video
        </Toolbar.Button>
      </Toolbar.Root>
      <p className="clip-live-note" role="status">
        {status(engine, frames !== null)}
      </p>
    </div>
  );
}

// A frame count as seconds, to the tenth.
function seconds(frames: number): string {
  return (frames / FPS).toFixed(1);
}

function status(engine: Engine, seeks: boolean): string {
  switch (engine.kind) {
    case "loading":
      return "Starting the game…";
    case "error":
      return `The game stopped: ${engine.error}`;
    case "playback": {
      const { ready, paused, live, ended } = engine.playback;
      // The game's default keys (its KeyConfig), which a scene's replay keeps.
      if (live) {
        const keys = "You have the game: arrow keys move, Z jumps, X attacks, Shift runs, A bombs, S ropes.";
        return seeks ? `${keys} The slider takes you back to the replay.` : keys;
      }
      if (!ready) return "Starting the game…";
      if (ended) return "The clip's over. Take control to play on from here.";
      if (paused) return "Paused.";
      return "Replaying the clip in the game.";
    }
  }
}
