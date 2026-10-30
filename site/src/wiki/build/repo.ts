// Links to the repo's files on GitHub (it's public): a replay a page cites, a docs/quirks.md row. Pinned, so that
// a link shows the file the page was checked against, and still does once the file changes or goes: to the last
// commit that changed the file, as of the one the site is built from (build.sh's WIKI_COMMIT), which has the file
// as that one does (filePins).
import { execFileSync } from "node:child_process";

export const REPO_URL = "https://github.com/851-labs/spelunky";

// Each file's pin: the last commit to change it, as of `commit`. Not `commit` itself, which every page's chunk would
// then have in its links, renaming each at every build (its hash is of its code) and with it the chunks that name
// it, so every deploy would throw away readers' cached scripts and the carried files would be most of a build's
// (851-3740). The last commit to change a file changes only with the file. `commit` where git can't say (a file
// it hasn't got, a commit it doesn't know: test-wiki.mjs's stand-in), and "main", unpinned, under `npm run dev`.
// Read once a repo and commit, for every load of the wiki that asks (test-wiki.mjs loads it many times).
const histories = new Map<string, Map<string, string>>();

export function filePins(repoDir: string, commit: string): (path: string) => string {
  const key = `${repoDir}\0${commit}`;
  // One pass over the history, newest first, read the first time it's asked (a few hundredths of a second): each
  // file's first commit in it is its last change. Not a `git log` a file, which took the build seconds. The first
  // parents only, as `git log -1 -- FILE` reads a merge (CI's checkout of a pull request merged into main): a
  // merge's own changes are its first parent's diff.
  const read = () => {
    const map = new Map<string, string>();
    const git = (...args: string[]) =>
      execFileSync("git", ["-C", repoDir, "-c", "core.quotePath=false", ...args], { encoding: "utf8", maxBuffer: 1 << 30, stdio: ["ignore", "pipe", "pipe"] });
    let log: string;
    try {
      log = git("log", "--first-parent", "--format=@%H", "--name-only", "--no-renames", commit);
    } catch (error) {
      // A commit git doesn't know (test-wiki.mjs's stand-in) links every file at it. One it knows whose history it
      // can't read fails the build, rather than quietly putting the commit back in every page's code.
      try {
        git("rev-parse", "--verify", "--quiet", `${commit}^{commit}`);
      } catch {
        return map;
      }
      throw new Error(`git can't read the history of ${commit}, which the wiki's links to the repo are pinned by: ${error}`);
    }
    let at = "";
    for (const line of log.split("\n")) {
      if (line.startsWith("@")) at = line.slice(1);
      else if (line && !map.has(line)) map.set(line, at);
    }
    return map;
  };
  return (path) => {
    if (commit === "main") return commit;
    let pins = histories.get(key);
    if (!pins) histories.set(key, (pins = read()));
    return pins.get(path) ?? commit;
  };
}

// A file, or a line of it: a Markdown file's line in its source (`?plain=1`), as GitHub renders the rest without
// line numbers.
export function repoUrl(commit: string, path: string, line?: number): string {
  const file = `${REPO_URL}/blob/${commit}/${path}`;
  if (line === undefined) return file;
  return `${file}${path.endsWith(".md") ? "?plain=1" : ""}#L${line}`;
}
