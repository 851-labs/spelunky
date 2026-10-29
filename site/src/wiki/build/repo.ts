// Links to the repo's files on GitHub (it's public): a replay a page cites, a docs/quirks.md row. Pinned to the
// commit the site is built from (build.sh's WIKI_COMMIT), so that a link shows the file the page was checked
// against, and still does once the file changes or goes.
export const REPO_URL = "https://github.com/851-labs/spelunky";

// A file, or a line of it: a Markdown file's line in its source (`?plain=1`), as GitHub renders the rest without
// line numbers.
export function repoUrl(commit: string, path: string, line?: number): string {
  const file = `${REPO_URL}/blob/${commit}/${path}`;
  if (line === undefined) return file;
  return `${file}${path.endsWith(".md") ? "?plain=1" : ""}#L${line}`;
}
