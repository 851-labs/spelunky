// The game in a clip's box (live-clip.tsx): /play/ in a same-origin iframe, replaying the clip, and Base UI's
// toolbar of controls under it. The page drives the game with postMessage, and the game tells it where its
// replay is (crates/app's web::listen_to_embedder and tell_embedder):
//
//   page to game  "spelunky:pause", "spelunky:play", "spelunky:take-control"
//   game to page  { spelunky: "playback", ready, paused, live, ended }, from its start, before its downloads
//                 { spelunky: "error", error }
//
// The game runs the replay's frames before the clip's unshown, as fast as they go, and says it's ready on the
// clip's first, so the poster stays over it until then. At the replay's end it holds the last frame, from which
// the reader can take over too. Restarting is loading the iframe again: /play/'s files are cached for good.
import { Toolbar } from "@base-ui/react/toolbar";
import { useEffect, useRef, useState } from "react";

interface Playback {
  ready: boolean;
  paused: boolean;
  live: boolean;
  ended: boolean;
}

type Engine =
  // Its page is loading, or the game is starting: nothing heard yet.
  | { kind: "loading" }
  | { kind: "playback"; playback: Playback }
  | { kind: "error"; error: string };

export interface LiveClipPlayerProps {
  title: string;
  width: string | number;
  height: string | number;
  poster?: string;
  replay: string;
  from: number;
  // Back to the video.
  onClose: () => void;
}

export function LiveClipPlayer({ title, width, height, poster, replay, from, onClose }: LiveClipPlayerProps) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [engine, setEngine] = useState<Engine>({ kind: "loading" });
  // A new iframe each time, for Restart.
  const [run, setRun] = useState(0);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      // Only this box's game: a page can have several.
      if (event.origin !== location.origin || event.source !== frame.current?.contentWindow) return;
      const data = event.data;
      if (data?.spelunky === "playback") setEngine({ kind: "playback", playback: data });
      else if (data?.spelunky === "error") setEngine({ kind: "error", error: String(data.error) });
    };
    addEventListener("message", listener);
    return () => removeEventListener("message", listener);
  }, []);

  // Whole, so that site/test-bundle.mjs can find them.
  const send = (message: "spelunky:pause" | "spelunky:play" | "spelunky:take-control") =>
    frame.current?.contentWindow?.postMessage(message, location.origin);
  const playback = engine.kind === "playback" ? engine.playback : null;
  const ready = playback?.ready ?? false;
  const live = playback?.live ?? false;
  const ended = playback?.ended ?? false;
  const paused = playback?.paused ?? false;
  const restart = () => {
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
      <Toolbar.Root className="clip-controls" aria-label={`${title}: the game's controls`}>
        <Toolbar.Button className="clip-button" disabled={!ready || live || ended} onClick={() => send(paused ? "spelunky:play" : "spelunky:pause")}>
          {paused && !ended ? "Play" : "Pause"}
        </Toolbar.Button>
        <Toolbar.Button className="clip-button" disabled={!ready || live} onClick={takeControl}>
          Take control
        </Toolbar.Button>
        <Toolbar.Separator className="clip-controls-gap" />
        <Toolbar.Button className="clip-button clip-button-quiet" onClick={restart}>
          Restart
        </Toolbar.Button>
        <Toolbar.Button className="clip-button clip-button-quiet" onClick={onClose}>
          Back to the video
        </Toolbar.Button>
      </Toolbar.Root>
      <p className="clip-live-note" role="status">
        {status(engine)}
      </p>
    </div>
  );
}

function status(engine: Engine): string {
  switch (engine.kind) {
    case "loading":
      return "Starting the game…";
    case "error":
      return `The game stopped: ${engine.error}`;
    case "playback": {
      const { ready, paused, live, ended } = engine.playback;
      // The game's default keys (its KeyConfig), which a scene's replay keeps.
      if (live) return "You have the game: arrow keys move, Z jumps, X attacks, Shift runs, A bombs, S ropes.";
      if (!ready) return "Starting the game…";
      if (ended) return "The clip's over. Take control to play on from here.";
      if (paused) return "Paused.";
      return "Replaying the clip in the game.";
    }
  }
}
