// The live clip player's bound on the game's start-up run (live-clip-player.tsx): the replay's frames before the
// clip's first (and the clip's own, for a game that runs it through first), which the game runs unshown, as fast as
// the device runs them, before it says it's ready. One that takes too long goes back to the video. Its own module, without React, so that test-wiki.mjs can test it.
//
// Kali's spiders, the latest clip (from frame 8,850), get there in 3.5 s on an M1 Pro in Chromium, 9 s with its CPU
// slowed twice, and 17 to 21 s slowed four times (Lighthouse's mid-range phone), so there every clip makes it, or
// all but the latest few; slowed eight times, it goes back 3 s into its run. (Timed on a busy machine.) Twenty
// seconds is long to look at a poster for a few seconds' clip, even one with a progress bar; the video's there.
export const START_MS = 20000;
// With the game's progress (its reports' `lead`), the player needn't wait that long to know: a run that's gone on
// for START_JUDGE_MS at a pace that would take it past START_MS, from the first report it counted, goes back then.
// The run's first frames (the boot, the title) go faster than the rest, so the pace only errs on the side of
// waiting.
export const START_JUDGE_MS = 3000;

// What of the game's playback reports the bound reads: whether the clip's first frame is showing, the clip's
// frames (a game that seeks says), and the replay's frames before the clip's first run so far (a game that reports
// its start-up run says).
export interface StartupReport {
  ready: boolean;
  frames?: number;
  lead?: number;
}

// The start-up run's time so far: whether it's begun, its milliseconds counted, and when last counted (null while
// not counting), the first report of its lead counted since the clock last started, and the latest, and whether
// it's decided.
export interface Startup {
  begun: boolean;
  ms: number;
  at: number | null;
  base: { ms: number; lead: number } | null;
  lead: number;
  decided: boolean;
}

// The lead a game's start-up run gets to as the clip's first frame shows: `from`, or for a game that runs the clip
// through before showing it (`states`, the clip's states_hash, which the page checks only against such a game), the
// replay's end, once the game's said how many frames the clip has.
export function leadGoal(from: number, states: string | null, report: StartupReport | null): number {
  return states !== null && report?.frames !== undefined ? from + report.frames : from;
}

export function newStartup(): Startup {
  return { begun: false, ms: 0, at: null, base: null, lead: 0, decided: false };
}

// Counts the start-up run's time to `now`, at a report or between them (null), while the game's in view of a
// visible page (`counting`), as a browser can slow one that isn't. Says whether it's taken too long: past
// START_MS, or, at a report with a lead, at a pace that would take it past that to `goal`, the lead at the clip's
// first frame (live-clip-player.tsx's leadGoal: `from`, or the replay's end for a game that runs the clip through
// first). Only reports judge the pace: between them, the lead is the last report's while the time goes on, which a
// moment's stall would make look slow.
//
// The clock starts at the first report of the clip's frames, which a game that seeks sends after its first slice
// of the run, once it's downloaded and set up its GPU, as a slow network is no slow device. (Its first report, as
// it starts, has none.) A game that doesn't seek never says, and its clock starts at its first report,
// downloads and all. It ends at the clip's first frame.
export function timeStartup(startup: Startup, report: StartupReport | null, counting: boolean, goal: number, now: number): boolean {
  if (startup.decided) return false;
  if (report?.ready) {
    startup.decided = true;
    return false;
  }
  if (startup.at !== null) startup.ms += now - startup.at;
  startup.begun ||= report !== null && (report.frames === undefined || report.frames > 0);
  startup.at = startup.begun && counting ? now : null;
  // Its pace counts from when the clock last started: the frames run while it was stopped weren't timed.
  if (startup.at === null) startup.base = null;
  const lead = startup.at !== null ? report?.lead : undefined;
  if (lead !== undefined) {
    startup.lead = lead;
    startup.base ??= { ms: startup.ms, lead };
  }
  const { ms, base } = startup;
  const paced = lead !== undefined && base !== null && ms - base.ms >= START_JUDGE_MS && lead > base.lead;
  const late = ms > START_MS || (paced && ms + ((goal - lead) * (ms - base.ms)) / (lead - base.lead) > START_MS);
  if (late) startup.decided = true;
  return late;
}
