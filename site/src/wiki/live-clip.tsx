// A clip in the game itself: a clip whose replay the media have (a scene's) has a "Play live" button under its
// video, which swaps the video for the web build at /play/ replaying the clip from its first frame, with
// controls to pause it and to take over from the replay (live-clip-player.tsx).
//
// Nothing of the game loads until the click: the page has only this button. The player is a chunk of its own,
// and the game is a same-origin iframe of /play/?replay=URL&from=F&embed (crates/app's window::Embed), made
// then, so it's /play/'s own page and assets, cached for good, and none of them is the page's JS. A phone keeps
// the video (wiki.css hides the button there): the game needs a keyboard, and it's megabytes to download.
import { Button } from "@base-ui/react/button";
import { useState } from "react";
import type { ReactNode } from "react";
import type { LiveClipPlayer } from "./live-clip-player";
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
  // The video.
  children: ReactNode;
}

export function LiveClip({ title, width, height, poster, replay, from, children }: LiveClipProps) {
  // The player, once its chunk is in: until then, and if it can't load, the video stays.
  const [Player, setPlayer] = useState<typeof LiveClipPlayer | null>(null);
  const [state, setState] = useState<"idle" | "loading" | "failed">("idle");
  if (Player) {
    return <Player title={title} width={width} height={height} poster={poster} replay={replay} from={from} onClose={() => setPlayer(null)} />;
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
      <div className="clip-live">
        <Button className="clip-button" onClick={start} disabled={state === "loading"} focusableWhenDisabled>
          Play live
        </Button>
        <span className="clip-live-note" role="status">
          {state === "failed" ? "Couldn't load the game's controls. Try again?" : "Replays it in the game, where you can take over."}
        </span>
      </div>
    </>
  );
}
