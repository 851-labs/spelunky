// GML syntax highlighting for the source viewer (source.ts), at build time: each line of a file as HTML, its
// tokens in <span>s whose one-letter classes wiki.css colours, so the browser gets markup and no highlighter.
// A lexer, not a parser: comments, strings, numbers, keywords, the constants GML spells as words, and a name
// before `(` as a call. Enough to read by, and it never changes the text: a line's HTML, unescaped and
// without its tags, is the line.
//
// GM8's GML has no escapes in strings (`"` or `'`, either may hold the other, and a newline), and comments as
// C's. A block comment or string left open at a line's end carries on into the next, each line's HTML whole,
// but not past the action: GM8 compiles each code action alone, so the extractor's line naming the next one
// starts afresh.

// The words GML reads as statements or operators.
const KEYWORDS = new Set([
  "if", "then", "else", "while", "do", "until", "for", "repeat", "switch", "case", "default", "break",
  "continue", "exit", "return", "with", "var", "globalvar", "and", "or", "xor", "not", "div", "mod",
  "begin", "end",
]);
// And the values it spells as words.
const LITERALS = new Set(["true", "false", "self", "other", "all", "noone", "global", "pi"]);

// k: keyword, l: literal, n: number, s: string, c: comment, f: a call's name, m: the extractor's line naming
// an action (`// ==== action 1: lib 1 id 603: code ====`), which isn't the game's own text.
type Kind = "k" | "l" | "n" | "s" | "c" | "f" | "m";

const ACTION_LINE = /^\/\/ ==== action \d+: .* ====$/;
const TOKEN = /\/\/|\/\*|"|'|\$[0-9A-Fa-f]+|\d+(?:\.\d*)?|\.\d+|[A-Za-z_][A-Za-z0-9_]*/g;

export function escapeHtml(text: string): string {
  return text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll(">", "&gt;");
}

const span = (kind: Kind, text: string) => (text === "" ? "" : `<span class="${kind}">${escapeHtml(text)}</span>`);

// The file's lines, highlighted. Lines split as the gml/ tree's are cited (wiki-cites): at each "\n", with no
// line after a final one.
export function highlightGml(source: string): string[] {
  const lines = source.split("\n");
  if (lines.at(-1) === "") lines.pop();
  // What's open at a line's start: a block comment, a string (its quote), or nothing.
  let open: "/*" | '"' | "'" | null = null;
  return lines.map((line) => {
    if (ACTION_LINE.test(line)) {
      open = null;
      return span("m", line);
    }
    let html = "";
    let at = 0;
    // Carries on what the line before left open, to its close or the line's end.
    const until = (close: string, kind: Kind, from: number) => {
      const end = line.indexOf(close, from);
      if (end < 0) {
        html += span(kind, line.slice(at));
        at = line.length;
        return false;
      }
      html += span(kind, line.slice(at, end + close.length));
      at = end + close.length;
      return true;
    };
    if (open !== null && until(open === "/*" ? "*/" : open, open === "/*" ? "c" : "s", 0)) open = null;
    if (open !== null) return html;
    TOKEN.lastIndex = at;
    for (let m = TOKEN.exec(line); m; m = TOKEN.exec(line)) {
      html += escapeHtml(line.slice(at, m.index));
      at = m.index;
      const token = m[0];
      if (token === "//") {
        html += span("c", line.slice(at));
        at = line.length;
        break;
      }
      if (token === "/*" || token === '"' || token === "'") {
        const close = token === "/*" ? "*/" : token;
        if (!until(close, token === "/*" ? "c" : "s", at + token.length)) {
          open = token;
          break;
        }
        TOKEN.lastIndex = at;
        continue;
      }
      let kind: Kind | null = null;
      if (/^[$\d.]/.test(token)) kind = "n";
      else if (KEYWORDS.has(token)) kind = "k";
      else if (LITERALS.has(token)) kind = "l";
      else if (/^\s*\(/.test(line.slice(at + token.length))) kind = "f";
      html += kind ? span(kind, token) : escapeHtml(token);
      at += token.length;
    }
    return html + escapeHtml(line.slice(at));
  });
}
