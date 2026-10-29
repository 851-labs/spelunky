// The game in a clip's box (live-clip.tsx): /play/ in a same-origin iframe, replaying the clip, and Base UI's
// slider and toolbar of controls under it. The page drives the game with postMessage, and the game tells it
// where its replay is (crates/app's web::listen_to_embedder and tell_embedder):
//
//   page to game  "spelunky:pause", "spelunky:play", "spelunky:take-control", { spelunky: "seek", frame }
//   game to page  { spelunky: "playback", ready, paused, live, ended, frame, frames, lead }, from its start,
//                 before its downloads
//                 { spelunky: "error", error }
//
// The game runs the replay's frames before the clip's unshown, as fast as they go, and says it's ready on the
// clip's first, so the poster stays over it until then. A game that reports that run (`progress`, build.sh's
// WIKI_LIVE_PROGRESS, from /play/'s page) says how many of them it's run (`lead`), which a Base UI Progress
// shows over the poster. The run is bounded either way: one that takes too long goes back to the video
// (`onSlow`, timeStartup). At the replay's end it holds the last frame, from which
// the reader can take over too. A game that seeks (`seek`, build.sh's WIKI_LIVE_SEEK, from /play/'s page) keeps
// savestates as it plays: the slider seeks in the clip, handing the game back to the replay if the reader has
// it, and Restart is a seek to its first frame. Without, it has no frame or frames, and Restarting is loading
// the iframe again: /play/'s files are cached for good.
//
// A touch screen has no Take control: the game needs a keyboard. And the player goes back to the video when
// the device can't keep the game up to speed: while the replay plays in view, its frames come too slowly
// (`onSlow`, live-clip.tsx), which the frames a seeking game reports tell.
import { Progress } from "@base-ui/react/progress";
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
  // The replay's frames before the clip's first run so far, up to `from`: a game that reports its start-up run
  // says.
  lead?: number;
}

type Engine =
  // Its page is loading, or the game is starting: nothing heard yet.
  | { kind: "loading" }
  | { kind: "playback"; playback: Playback }
  | { kind: "error"; error: string };

// The game's 30 frames a second.
const FPS = 30;
// Too slow: frames that come at under two thirds of that, as the median over three seconds (and ten reports) of
// the replay playing in view. The median, so that a moment's hitch doesn't decide.
const SLOW_FPS = 20;
const WATCH_MS = 3000;
const WATCH_REPORTS = 10;
// The frames not timed from the clip's first, the game's first moments (its shaders, the sound starting), and
// from a seek's, which comes after the frames run unshown to it.
const SETTLE_START = 15;
const SETTLE_SEEK = 1;
// The ticks a game that's fallen behind runs back to back, so between two reports (crates/app's
// window::MAX_CATCH_UP).
const CATCH_UP = 4;
// The start-up run's bound: the replay's frames before the clip's first, run unshown, as fast as the device runs
// them. Kali's spiders, the latest clip (from frame 8,850), get there in 3.3 s on an M1 Pro in Chromium, 10 s with
// its CPU slowed twice, and 27 to 32 s slowed four times (Lighthouse's mid-range phone), where every clip from
// before about frame 6,000 makes it: all but 17 of the 276. Twenty seconds is long to look at a poster for a few
// seconds' clip, even one with a progress bar; the video's there. The clock starts at the run's first report,
// which comes once the game's downloaded and its GPU's set up, as a slow network is no slow device, and counts
// only while the game's in view of a visible page, as a browser can slow one that isn't.
const START_MS = 20000;
// With the game's progress, the player needn't wait that long to know: a run that's gone on for START_JUDGE_MS
// at a pace that would take it past START_MS, from the first report it counted, goes back then. The run's first
// frames (the boot, the title) go faster than the rest, so the pace only errs on the side of waiting.
const START_JUDGE_MS = 3000;
// Wiki.css's touch screens, which hide the button where the game can't play on phones.
const TOUCH = "(hover: none), (pointer: coarse)";

export interface LiveClipPlayerProps {
  title: string;
  width: string | number;
  height: string | number;
  poster?: string;
  replay: string;
  from: number;
  // The game seeks: a slider, and an instant Restart.
  seek: boolean;
  // The game reports its start-up run: a progress bar over the poster.
  progress: boolean;
  // Back to the video.
  onClose: () => void;
  // Back to the video, as the device can't keep up (`slow`) or can't get to the clip's first frame in time
  // (`late`).
  onSlow: (why: Slow) => void;
}

export type Slow = "slow" | "late";

export function LiveClipPlayer({ title, width, height, poster, replay, from, seek, progress, onClose, onSlow }: LiveClipPlayerProps) {
  const frame = useRef<HTMLIFrameElement>(null);
  const [engine, setEngine] = useState<Engine>({ kind: "loading" });
  // A new iframe each time, for Restart without seeking.
  const [run, setRun] = useState(0);
  // Where the reader put the slider, until the game reports that frame (or a moment has passed, whatever it
  // reports): its reports from before it had the seek would pull the thumb back.
  const [scrub, setScrub] = useState<number | null>(null);
  const scrubTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  useEffect(() => () => clearTimeout(scrubTimer.current), []);
  const [touch] = useState(() => matchMedia(TOUCH).matches);
  // The replay's frames as they came, while it plays in view, to tell whether the device keeps up (timeFrames).
  // A browser can slow a frame that's out of view or a hidden tab, so only one in view counts, and the count
  // starts again from the next frame when it comes back.
  const speed = useRef<Speed>({ perFrame: [], ms: 0, frame: null, at: 0, settle: SETTLE_START, decided: false });
  // The start-up run's time so far, to bound it (timeStartup). A new iframe runs it again.
  const startup = useRef<Startup>(newStartup());
  const inView = useRef(true);
  const counting = () => inView.current && document.visibilityState === "visible";
  const slow = useRef(onSlow);
  slow.current = onSlow;
  useEffect(() => {
    const element = frame.current;
    if (!element) return;
    const away = () => {
      speed.current.frame = null;
      timeStartup(startup.current, null, counting(), from, slow.current);
    };
    const observer = new IntersectionObserver(
      ([entry]) => {
        inView.current = entry.isIntersecting;
        away();
      },
      { threshold: 0.5 },
    );
    observer.observe(element);
    document.addEventListener("visibilitychange", away);
    return () => {
      observer.disconnect();
      document.removeEventListener("visibilitychange", away);
    };
    // Restart without seeking makes a new iframe.
  }, [run]);

  useEffect(() => {
    const listener = (event: MessageEvent) => {
      // Only this box's game: a page can have several.
      if (event.origin !== location.origin || event.source !== frame.current?.contentWindow) return;
      const data = event.data;
      if (data?.spelunky === "playback") {
        timeFrames(speed.current, data, counting(), slow.current);
        timeStartup(startup.current, data, counting(), from, slow.current);
        setEngine({ kind: "playback", playback: data });
        setScrub((at) => (at === data.frame ? null : at));
      } else if (data?.spelunky === "error") setEngine({ kind: "error", error: String(data.error) });
    };
    addEventListener("message", listener);
    return () => removeEventListener("message", listener);
  }, [from]);

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
    speed.current.frame = null;
    speed.current.settle = Math.max(speed.current.settle, SETTLE_SEEK);
  };
  const restart = () => {
    if (frames !== null) {
      seekTo(0);
      send("spelunky:play");
      return;
    }
    setEngine({ kind: "loading" });
    startup.current = newStartup();
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
  // The start-up run's clock runs between reports too, which a device that's stuck sends none of.
  useEffect(() => {
    if (!covered) return;
    const timer = setInterval(() => timeStartup(startup.current, null, counting(), from, slow.current), START_CHECK_MS);
    return () => clearInterval(timer);
  }, [covered, from, run]);
  // Its frames run so far, once it's begun: until then, the game's downloading and setting up.
  const lead = progress && playback?.frames && playback.lead !== undefined ? playback.lead : null;
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
            {progress ? (
              <Progress.Root className="clip-progress" value={lead} max={from}>
                <Progress.Label className="clip-progress-label">Starting the game…</Progress.Label>
                <Progress.Track className="clip-progress-track">
                  <Progress.Indicator className="clip-progress-indicator" />
                </Progress.Track>
              </Progress.Root>
            ) : (
              <span>Starting the game…</span>
            )}
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
        {!touch && (
          <Toolbar.Button className="clip-button" disabled={!ready || live} onClick={takeControl}>
            Take control
          </Toolbar.Button>
        )}
        <Toolbar.Separator className="clip-controls-gap" />
        <Toolbar.Button className="clip-button clip-button-quiet" disabled={frames !== null && !ready} onClick={restart}>
          Restart
        </Toolbar.Button>
        <Toolbar.Button className="clip-button clip-button-quiet" onClick={onClose}>
          Back to the video
        </Toolbar.Button>
      </Toolbar.Root>
      <p className="clip-live-note" role="status">
        {status(engine, frames !== null, touch)}
      </p>
    </div>
  );
}

// A frame count as seconds, to the tenth.
function seconds(frames: number): string {
  return (frames / FPS).toFixed(1);
}

// The replay's frames' timing so far (`speed` in LiveClipPlayer): the milliseconds each frame took, and their
// sum, the last report's frame (null when it wasn't counted) and time, the frames still to settle before they
// count, and whether it's decided.
interface Speed {
  perFrame: number[];
  ms: number;
  frame: number | null;
  at: number;
  settle: number;
  decided: boolean;
}

// Times a playback report's frame, if the replay plays in view, and calls `slow` if enough came too slowly.
// Only a frame just after the last one counted counts, at most CATCH_UP after it: not a seek's jump, nor the
// first after a pause or a spell out of view.
function timeFrames(speed: Speed, playback: Playback, inView: boolean, slow: (why: Slow) => void) {
  if (speed.decided) return;
  const now = performance.now();
  const playing = inView && playback.ready && !playback.paused && !playback.live && !playback.ended;
  const frame = playing ? (playback.frame ?? null) : null;
  const step = frame !== null && speed.frame !== null ? frame - speed.frame : 0;
  if (step >= 1 && step <= CATCH_UP) {
    if (speed.settle > 0) speed.settle--;
    else {
      speed.perFrame.push((now - speed.at) / step);
      speed.ms += now - speed.at;
    }
  }
  speed.frame = frame;
  speed.at = now;
  if (speed.ms < WATCH_MS || speed.perFrame.length < WATCH_REPORTS) return;
  speed.decided = true;
  const sorted = [...speed.perFrame].sort((a, b) => a - b);
  if (sorted[sorted.length >> 1] > 1000 / SLOW_FPS) slow("slow");
}

// The start-up run's time so far (`startup` in LiveClipPlayer): whether it's begun, its milliseconds counted, and
// when last counted (null while not counting), the first report of its lead counted since it last started
// counting, and the latest, and whether it's decided.
interface Startup {
  begun: boolean;
  ms: number;
  at: number | null;
  base: { ms: number; lead: number } | null;
  lead: number;
  decided: boolean;
}

function newStartup(): Startup {
  return { begun: false, ms: 0, at: null, base: null, lead: 0, decided: false };
}

// How often the start-up run's clock is checked between reports.
const START_CHECK_MS = 250;

// Counts the start-up run's time to now, from a report (`playback`) or between them (null), while the game's
// in view of a visible page (`counting`), and calls `late` if it's taken too long: START_MS, or, with the game's
// lead, a pace that would take it past that to `from`. It begins at the first report of the clip's frames, which
// a game sends once its run has begun, and ends at the clip's first frame.
function timeStartup(startup: Startup, playback: Playback | null, counting: boolean, from: number, late: (why: Slow) => void) {
  if (startup.decided) return;
  if (playback?.ready) {
    startup.decided = true;
    return;
  }
  const now = performance.now();
  if (startup.at !== null) startup.ms += now - startup.at;
  startup.begun ||= !!playback?.frames;
  startup.at = startup.begun && counting ? now : null;
  // Its pace counts from when the clock last started: the frames run while it was stopped weren't timed.
  if (startup.at === null) startup.base = null;
  else if (playback?.lead !== undefined) {
    startup.lead = playback.lead;
    startup.base ??= { ms: startup.ms, lead: playback.lead };
  }
  const { ms, base, lead } = startup;
  const paced = base !== null && ms - base.ms >= START_JUDGE_MS && lead > base.lead;
  if (ms > START_MS || (paced && ms + ((from - lead) * (ms - base.ms)) / (lead - base.lead) > START_MS)) {
    startup.decided = true;
    late("late");
  }
}

function status(engine: Engine, seeks: boolean, touch: boolean): string {
  switch (engine.kind) {
    case "loading":
      return "Starting the game…";
    case "error":
      return `The game stopped: ${engine.error}`;
    case "playback": {
      const { ready, paused, live, ended } = engine.playback;
      // The game's default keys (its KeyConfig), which every clip's replay keeps (harness's clip tests check).
      if (live) {
        const keys = "You have the game: arrow keys move, Z jumps, X attacks, Shift runs, A bombs, S ropes.";
        return seeks ? `${keys} The slider takes you back to the replay.` : keys;
      }
      if (!ready) return "Starting the game…";
      if (ended) return touch ? "The clip's over." : "The clip's over. Take control to play on from here.";
      if (paused) return "Paused.";
      return "Replaying the clip in the game.";
    }
  }
}
