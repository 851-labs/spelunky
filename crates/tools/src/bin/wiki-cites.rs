//! Checks the wiki's pages against the game's code they cite: every path in the `gml/` tree a page names
//! must be a file there, every line range must be inside it, and every ```` ```gml ```` excerpt must be
//! exactly the lines its info string cites (whitespace aside).
//!
//! ```sh
//! wiki-cites --assets assets/ site/content/wiki/        # or some pages: site/content/wiki/snake.mdx
//! ```
//!
//! `scripts/wiki-cites` runs it with those defaults, and `scripts/check` runs it on every pull request. The
//! site's build can't: Workers Builds has no extracted assets, so it checks a citation's form
//! (`objects/oSnake/step.gml:41-54`) but not that the lines exist or hold what a page quotes
//! (`docs/wiki-style.md`, "Citing the code").
//!
//! # What it reads
//!
//! Rather than parse a page's YAML and MDX, it scans each line for the paths the site's schema allows
//! (`objects/...gml` and `scripts/...gml`, with `:N` or `:N-M` for lines), wherever they are: a `<Cite gml>`,
//! an infobox's or a relation's `cite`, `sources`, a quirk's `where`, an excerpt's info string, a comment
//! or the prose. A citation can't hide in syntax the scan doesn't know, and a path that's wrong in a
//! comment is wrong too. Only the excerpts need their structure: a fenced block whose info string starts
//! with `gml`.
//!
//! The tree is the one `scripts/extract` writes (`tools::gml_tree`'s layout), read from disk with its
//! files' own names, so a path whose case is wrong fails here as it would on a case-sensitive server.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use tools::{Arg, Args, write_stdout};

const USAGE: &str = "usage: wiki-cites --assets <extracted bundle dir> <page.mdx | page dir>...";

fn main() -> ExitCode {
    match run(std::env::args().skip(1)) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(e) => {
            eprintln!("wiki-cites: {e}");
            ExitCode::from(2)
        }
    }
}

/// Checks the pages, printing what's wrong; `Ok(false)` if anything is.
fn run(args: impl IntoIterator<Item = String>) -> Result<bool, String> {
    let mut args = Args::new(args);
    let (mut assets, mut inputs) = (None, Vec::new());
    while let Some(arg) = args.next() {
        let flag = match arg {
            Arg::Flag(flag) => flag,
            Arg::Positional(p) => {
                inputs.push(PathBuf::from(p));
                continue;
            }
        };
        match flag.name.as_str() {
            "-h" | "--help" => {
                args.switch(&flag)?;
                write_stdout(&format!("{USAGE}\n"))?;
                return Ok(true);
            }
            "--assets" => assets = Some(PathBuf::from(args.value(&flag)?)),
            f => return Err(format!("unknown option {f}\n{USAGE}")),
        }
    }
    let assets = assets.ok_or(format!("--assets is required\n{USAGE}"))?;
    if inputs.is_empty() {
        return Err(format!("no pages given\n{USAGE}"));
    }
    let tree = Tree::read(&assets.join("gml"))?;
    let mut pages = Vec::new();
    for input in inputs {
        if input.is_dir() {
            let mut dir: Vec<PathBuf> = fs::read_dir(&input)
                .map_err(|e| format!("{}: {e}", input.display()))?
                .map(|entry| entry.map(|e| e.path()))
                .collect::<Result<_, _>>()
                .map_err(|e| format!("{}: {e}", input.display()))?;
            dir.retain(|p| p.extension().is_some_and(|e| e == "mdx"));
            dir.sort();
            pages.extend(dir);
        } else {
            pages.push(input);
        }
    }
    let (mut total, mut bad_pages, mut problems) = (Report::default(), 0, String::new());
    for page in &pages {
        let text = fs::read_to_string(page).map_err(|e| format!("{}: {e}", page.display()))?;
        let report = check_page(&text, &tree);
        for (line, problem) in &report.problems {
            problems.push_str(&format!("{}:{line}: {problem}\n", page.display()));
        }
        bad_pages += usize::from(!report.problems.is_empty());
        total.files += report.files;
        total.citations += report.citations;
        total.excerpts += report.excerpts;
        total.problems.extend(report.problems);
    }
    if total.problems.is_empty() {
        write_stdout(&format!(
            "wiki-cites: {}: {}, {} and {} match the GML\n",
            count(pages.len(), "page"),
            count(total.citations, "citation"),
            count(total.files, "file reference"),
            count(total.excerpts, "excerpt")
        ))?;
        return Ok(true);
    }
    eprint!("{problems}");
    eprintln!(
        "wiki-cites: {} on {bad_pages} of {} (paths are relative to {})",
        count(total.problems.len(), "problem"),
        count(pages.len(), "page"),
        assets.join("gml").display()
    );
    Ok(false)
}

/// The `gml/` tree: each file's lines by its path in the tree (`objects/oSnake/step.gml`).
struct Tree(BTreeMap<String, Vec<String>>);

impl Tree {
    fn read(root: &Path) -> Result<Tree, String> {
        if !root.is_dir() {
            return Err(format!(
                "no GML tree at {}: run scripts/extract first",
                root.display()
            ));
        }
        let mut files = BTreeMap::new();
        let mut dirs = vec![(root.to_path_buf(), String::new())];
        while let Some((dir, prefix)) = dirs.pop() {
            for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
                let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let path = format!("{prefix}{name}");
                if entry.path().is_dir() {
                    dirs.push((entry.path(), format!("{path}/")));
                } else if name.ends_with(".gml") {
                    let bytes = fs::read(entry.path())
                        .map_err(|e| format!("{}: {e}", entry.path().display()))?;
                    files.insert(path, lines(&String::from_utf8_lossy(&bytes)));
                }
            }
        }
        Ok(Tree(files))
    }

    /// Why `path` isn't in the tree, with what is: its name in the tree's case, or the files beside it.
    fn missing(&self, path: &str) -> String {
        let lower = path.to_ascii_lowercase();
        if let Some(found) = self.0.keys().find(|p| p.to_ascii_lowercase() == lower) {
            return format!("{path} isn't in the gml/ tree (did you mean {found}?)");
        }
        let dir = path.rsplit_once('/').map_or("", |(dir, _)| dir);
        let beside: Vec<&str> = self
            .0
            .keys()
            .filter_map(|p| p.strip_prefix(dir)?.strip_prefix('/'))
            .filter(|rest| !rest.contains('/'))
            .collect();
        if beside.is_empty() || dir == "objects" || dir == "scripts" {
            format!("{path} isn't in the gml/ tree")
        } else {
            format!(
                "{path} isn't in the gml/ tree ({dir}/ has {})",
                beside.join(", ")
            )
        }
    }
}

/// A file's lines, without their line endings.
fn lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_owned())
        .collect()
}

/// A path in the gml/ tree a page names, as written, with the lines it cites, if any.
#[derive(Debug, PartialEq)]
struct Reference<'a> {
    text: &'a str,
    path: &'a str,
    lines: Option<(usize, usize)>,
}

/// Every path in the gml/ tree `line` names: `objects/` or `scripts/` (not inside a longer name, but a
/// `gml/` before it is allowed, as in `assets/gml/objects/...`), then the characters the site's schema
/// allows a path, then `.gml`, then any `:N` or `:N-M`. `scripts/check` and the like don't end in `.gml`,
/// so they aren't references.
fn references(line: &str) -> Vec<Reference<'_>> {
    let bytes = line.as_bytes();
    let path_char = |b: u8| b.is_ascii_alphanumeric() || b == b'_' || b == b'/';
    let digits = |from: usize| {
        let n = bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count();
        (n > 0).then(|| (line[from..from + n].parse::<usize>().ok(), from + n))
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        // Bytes, not chars: `i` steps through multi-byte characters (a page's dashes) one byte at a time.
        let rest = &bytes[i..];
        let starts = (rest.starts_with(b"objects/") || rest.starts_with(b"scripts/"))
            && (i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_'));
        if !starts {
            i += 1;
            continue;
        }
        let mut end = i + bytes[i..].iter().take_while(|&&b| path_char(b)).count();
        if !bytes[end..].starts_with(b".gml") || bytes.get(end + 4).is_some_and(|&b| path_char(b)) {
            i = end;
            continue;
        }
        end += 4;
        let path_end = end;
        let mut lines = None;
        if bytes.get(end) == Some(&b':')
            && let Some((first, after)) = digits(end + 1)
        {
            end = after;
            let mut last = first;
            if bytes.get(end) == Some(&b'-')
                && let Some((to, after)) = digits(end + 1)
            {
                (last, end) = (to, after);
            }
            // A number too long for usize is out of range, as line 0 is: both fail as such.
            lines = Some((first.unwrap_or(usize::MAX), last.unwrap_or(usize::MAX)));
        }
        out.push(Reference {
            text: &line[i..end],
            path: &line[i..path_end],
            lines,
        });
        i = end;
    }
    out
}

/// What checking a page found: its problems (by its 1-based line) and what it checked.
#[derive(Default, Debug)]
struct Report {
    problems: Vec<(usize, String)>,
    citations: usize,
    files: usize,
    excerpts: usize,
}

/// A ```` ```gml ```` excerpt: the page's line its fence opens on, its info string after `gml`, and its
/// text, line by line.
struct Excerpt<'a> {
    line: usize,
    cite: &'a str,
    code: Vec<&'a str>,
}

fn check_page(text: &str, tree: &Tree) -> Report {
    let mut report = Report::default();
    let page: Vec<&str> = text.lines().collect();
    // The frontmatter is YAML, which has no code fences: only the body's are read as such.
    let body = match page.first() {
        Some(first) if first.trim_end() == "---" => page
            .iter()
            .skip(1)
            .position(|l| l.trim_end() == "---")
            .map_or(page.len(), |i| i + 2),
        _ => 0,
    };
    let mut excerpts = Vec::new();
    let mut scanned = vec![true; page.len()];
    let mut i = body;
    while i < page.len() {
        let Some((fence, info)) = fence_open(page[i]) else {
            if excerpt_not_read(page[i]) {
                report.problems.push((
                    i + 1,
                    "a ```gml excerpt this check can't read: open its fence on a line of its own \
                     (not in a quote or on a list item's first line)"
                        .into(),
                ));
            }
            i += 1;
            continue;
        };
        let close = (i + 1..page.len()).find(|&j| fence_closes(page[j], fence));
        let end = close.unwrap_or(page.len());
        if let Some(cite) = info
            .strip_prefix("gml")
            .filter(|rest| rest.is_empty() || rest.starts_with(char::is_whitespace))
        {
            if close.is_none() {
                report
                    .problems
                    .push((i + 1, "this ```gml excerpt's fence is never closed".into()));
            }
            // Its code is GML, not the page's: it isn't scanned for paths.
            scanned[i + 1..end].fill(false);
            excerpts.push(Excerpt {
                line: i + 1,
                cite: cite.trim(),
                code: page[i + 1..end].to_vec(),
            });
        }
        i = end + 1;
    }

    for (n, line) in page.iter().enumerate() {
        if !scanned[n] {
            continue;
        }
        for reference in references(line) {
            match reference.lines {
                Some(_) => report.citations += 1,
                None => report.files += 1,
            }
            if let Err(problem) = resolve(&reference, tree) {
                report.problems.push((n + 1, problem));
            }
        }
    }

    for excerpt in excerpts {
        report.excerpts += 1;
        let cited = match references(excerpt.cite).as_slice() {
            [r] if r.text == excerpt.cite && r.lines.is_some() => resolve(r, tree).ok(),
            _ => {
                report.problems.push((
                    excerpt.line,
                    format!(
                        "{}: an excerpt cites its lines after `gml` (```gml objects/oSnake/step.gml:73-76)",
                        format!("```gml {}", excerpt.cite).trim_end()
                    ),
                ));
                continue;
            }
        };
        // A citation that doesn't resolve has been reported where it's written, the fence's line.
        let Some((first, held)) = cited else { continue };
        if let Some(problem) = compare(excerpt.cite, first, held, &excerpt.code) {
            report.problems.push((excerpt.line, problem));
        }
    }
    report.problems.sort_by_key(|(line, _)| *line);
    report
}

/// The fence `line` opens (its character and length) and its info string, if it opens one. Any indent
/// is allowed, as MDX has no indented code blocks: a fence in a list item is indented with it.
fn fence_open(line: &str) -> Option<((char, usize), &str)> {
    let rest = line.trim_start();
    let c = rest.chars().next().filter(|&c| c == '`' || c == '~')?;
    let n = rest.chars().take_while(|&ch| ch == c).count();
    let info = rest[n..].trim();
    // A backtick fence's info string can't hold a backtick: ```` ```gml ...``` ```` in a line is inline code.
    (n >= 3 && !(c == '`' && info.contains('`'))).then_some(((c, n), info))
}

fn fence_closes(line: &str, (c, n): (char, usize)) -> bool {
    let rest = line.trim();
    rest.chars().all(|ch| ch == c) && rest.chars().count() >= n
}

/// Whether `line` opens a ```` ```gml ```` excerpt behind a quote's `>` or a list marker, where
/// [`fence_open`] doesn't look: such an excerpt would otherwise go unchecked.
fn excerpt_not_read(line: &str) -> bool {
    let rest = line.trim_start_matches(|c: char| {
        c.is_whitespace() || matches!(c, '>' | '-' | '*' | '+' | '.' | ')') || c.is_ascii_digit()
    });
    rest.len() < line.trim_start().len()
        && ["```gml", "~~~gml"].iter().any(|f| {
            rest.strip_prefix(f)
                .is_some_and(|r| r.is_empty() || r.starts_with(char::is_whitespace))
        })
}

/// Checks a reference against the tree: its file, and its lines inside it. Returns the first line's
/// number and the lines, for an excerpt to compare.
fn resolve<'t>(reference: &Reference, tree: &'t Tree) -> Result<(usize, &'t [String]), String> {
    let Some(file) = tree.0.get(reference.path) else {
        return Err(tree.missing(reference.path));
    };
    let Some((first, last)) = reference.lines else {
        return Ok((1, file));
    };
    if first == 0 || last < first {
        return Err(format!(
            "{}: lines are numbered from 1, first to last",
            reference.text
        ));
    }
    if last > file.len() {
        return Err(format!(
            "{}: {} has {}",
            reference.text,
            reference.path,
            count(file.len(), "line")
        ));
    }
    Ok((first, &file[first - 1..last]))
}

/// `n` of `thing`, in the plural unless it's one.
fn count(n: usize, thing: &str) -> String {
    match n {
        1 => format!("1 {thing}"),
        n => format!("{n} {thing}s"),
    }
}

/// A line with its whitespace normalised: trimmed, and each run inside it one space, so an excerpt may
/// indent with spaces what the GML indents with tabs.
fn normalise(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Why an excerpt isn't the lines it cites (`held`, from line `first`), if it isn't.
fn compare(cite: &str, first: usize, held: &[String], code: &[&str]) -> Option<String> {
    let (want, got): (Vec<String>, Vec<String>) = (
        held.iter().map(|l| normalise(l)).collect(),
        code.iter().map(|l| normalise(l)).collect(),
    );
    if want == got {
        return None;
    }
    let path = cite.split_once(':').map_or(cite, |(path, _)| path);
    let what = match (0..want.len().max(got.len())).find(|&k| want.get(k) != got.get(k)) {
        Some(k) if k < want.len() && k < got.len() => format!(
            "its line {} is `{}` where {path}:{} is `{}`",
            k + 1,
            got[k],
            first + k,
            want[k]
        ),
        _ => format!(
            "it has {} and the citation {}",
            count(got.len(), "line"),
            count(want.len(), "line")
        ),
    };
    let quoted: String = held.iter().map(|l| format!("\n    | {l}")).collect();
    Some(format!(
        "```gml {cite}: the excerpt isn't those lines: {what}. They hold:{quoted}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> Tree {
        let step = "\t// a comment\n\tif (status == ATTACK)\n\t{\n\t    // ?\n\t}\n";
        Tree(BTreeMap::from([
            ("objects/oSnake/step.gml".to_owned(), lines(step)),
            ("objects/oSnake/create.gml".to_owned(), lines("hp = 1;\n")),
            ("scripts/scrInit.gml".to_owned(), lines("a = 1;\nb = 2;\n")),
        ]))
    }

    fn problems(page: &str) -> Vec<(usize, String)> {
        check_page(page, &tree()).problems
    }

    #[test]
    fn finds_references_anywhere_in_a_line() {
        let found = references(
            r#"<Cite gml="objects/oSnake/step.gml:41-54" /> cite: scripts/scrInit.gml:2, in assets/gml/objects/oSnake/create.gml"#,
        );
        assert_eq!(
            found,
            [
                Reference {
                    text: "objects/oSnake/step.gml:41-54",
                    path: "objects/oSnake/step.gml",
                    lines: Some((41, 54))
                },
                Reference {
                    text: "scripts/scrInit.gml:2",
                    path: "scripts/scrInit.gml",
                    lines: Some((2, 2))
                },
                Reference {
                    text: "objects/oSnake/create.gml",
                    path: "objects/oSnake/create.gml",
                    lines: None
                },
            ]
        );
        // Not GML paths: other files, longer names, a path that goes on past `.gml`, a dangling range.
        assert_eq!(
            references("scripts/check, myobjects/a.gml, objects/a.gmlx, docs/scripts/x.md"),
            []
        );
        assert_eq!(references("objects/a.gml:3-")[0].lines, Some((3, 3)));
        assert_eq!(references("objects/a.gml: the step")[0].lines, None);
        assert_eq!(
            references("levels 1–4 (scripts/a.gml:2)")[0].text,
            "scripts/a.gml:2"
        );
    }

    #[test]
    fn a_good_page_passes() {
        let page = "---
slug: snake
infobox:
  behaviour: { text: \"Patrols\", cite: objects/oSnake/step.gml:1-5 }
relations:
  damaged_by:
    - { slug: bomb, cite: \"scripts/scrInit.gml:2\" }
sources:
  - objects/oSnake/create.gml
---

It bites <Cite gml=\"objects/oSnake/create.gml:1\" />.

- Its attack state is empty:

  ```gml objects/oSnake/step.gml:2-5
  if (status == ATTACK)
  {
      // ?
  }
  ```
";
        let report = check_page(page, &tree());
        assert_eq!(report.problems, []);
        assert_eq!((report.citations, report.files, report.excerpts), (4, 1, 1));
    }

    #[test]
    fn a_broken_page_fails_with_what_is_there() {
        let page = "---
sources:
  - objects/oSnake/Step.gml
  - objects/oSnake/draw.gml
---
<Cite gml=\"objects/oSnake/step.gml:4-9\" /> <Cite gml='scripts/scrInit.gml:0' />

```gml objects/oSnake/step.gml:2-5
if (status == ATTACK)
{
    // !
}
```

```gml objects/oSnake/step.gml:2-3
if (status == ATTACK)
```

> ```gml objects/oSnake/step.gml:2
";
        let problems = problems(page);
        let at = |line: usize| -> Vec<&str> {
            problems
                .iter()
                .filter(|(l, _)| *l == line)
                .map(|(_, p)| p.as_str())
                .collect()
        };
        assert_eq!(
            at(3),
            [
                "objects/oSnake/Step.gml isn't in the gml/ tree (did you mean objects/oSnake/step.gml?)"
            ]
        );
        assert_eq!(
            at(4),
            [
                "objects/oSnake/draw.gml isn't in the gml/ tree (objects/oSnake/ has create.gml, step.gml)"
            ]
        );
        assert_eq!(
            at(6),
            [
                "objects/oSnake/step.gml:4-9: objects/oSnake/step.gml has 5 lines",
                "scripts/scrInit.gml:0: lines are numbered from 1, first to last",
            ]
        );
        assert_eq!(
            at(8),
            [
                "```gml objects/oSnake/step.gml:2-5: the excerpt isn't those lines: its line 3 is `// !` \
              where objects/oSnake/step.gml:4 is `// ?`. They hold:\n    | \tif (status == ATTACK)\n    \
              | \t{\n    | \t    // ?\n    | \t}"
            ]
        );
        assert_eq!(
            at(15),
            [
                "```gml objects/oSnake/step.gml:2-3: the excerpt isn't those lines: it has 1 line and the \
              citation 2 lines. They hold:\n    | \tif (status == ATTACK)\n    | \t{"
            ]
        );
        assert_eq!(at(19).len(), 1);
        assert!(at(19)[0].contains("can't read"), "{problems:?}");
        assert_eq!(problems.len(), 7, "{problems:?}");
    }

    #[test]
    fn an_excerpt_needs_its_citation_and_its_fence() {
        assert_eq!(
            problems("```gml\nhp = 1;\n```\n"),
            [(
                1,
                "```gml: an excerpt cites its lines after `gml` (```gml objects/oSnake/step.gml:73-76)"
                    .to_owned()
            )]
        );
        assert_eq!(
            problems("```gml objects/oSnake/create.gml:1\nhp = 1;\n"),
            [(1, "this ```gml excerpt's fence is never closed".to_owned())]
        );
        // Other code blocks aren't excerpts, and a longer fence holds a shorter one.
        assert_eq!(
            problems(
                "````md\n```gml objects/oSnake/create.gml:1\nhp = 2;\n```\n````\n```sh\nls\n```\n"
            ),
            []
        );
    }
}
