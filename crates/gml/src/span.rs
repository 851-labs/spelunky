//! Source positions. Spans are byte offsets into the unit's source text, so a trace divergence in
//! translated code can be mapped back to the exact GML line it came from.

use std::fmt;

/// A half-open byte range `start..end` in one unit's source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        // GML units are at most a few tens of kilobytes; u32 keeps AST nodes small.
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    /// The smallest span covering both `self` and `other`.
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn text(self, source: &str) -> &str {
        &source[self.start as usize..self.end as usize]
    }
}

/// A 1-based line and column (columns count bytes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineCol {
    pub line: u32,
    pub col: u32,
}

impl fmt::Display for LineCol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

/// Maps byte offsets to lines. GM8 code uses CRLF; `\r` is treated as ordinary whitespace, so lines are
/// counted by `\n` and the result is the same for CRLF and LF text.
#[derive(Clone, Debug)]
pub struct LineIndex {
    line_starts: Vec<u32>,
}

impl LineIndex {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(
            source
                .bytes()
                .enumerate()
                .filter(|&(_, b)| b == b'\n')
                .map(|(i, _)| i as u32 + 1),
        );
        Self { line_starts }
    }

    pub fn line_col(&self, offset: u32) -> LineCol {
        let line = self.line_starts.partition_point(|&start| start <= offset);
        LineCol {
            line: line as u32,
            col: offset - self.line_starts[line - 1] + 1,
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_col_handles_crlf_and_lf() {
        let src = "a\r\nbc\nd";
        let idx = LineIndex::new(src);
        assert_eq!(idx.line_col(0), LineCol { line: 1, col: 1 });
        assert_eq!(idx.line_col(3), LineCol { line: 2, col: 1 });
        assert_eq!(idx.line_col(4), LineCol { line: 2, col: 2 });
        assert_eq!(idx.line_col(6), LineCol { line: 3, col: 1 });
        assert_eq!(idx.line_count(), 3);
    }
}
