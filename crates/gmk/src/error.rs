use std::fmt;

/// A failure to read a `.gmk` file.
///
/// `context` names the block being read (for example `sprite[12]`) and `offset` is the byte offset
/// within that block. Blocks that GameMaker stores zlib-compressed are counted from the start of their
/// inflated data, not from the start of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub context: String,
    pub offset: usize,
    pub kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    /// The file doesn't start with GameMaker's magic number.
    BadMagic(i32),
    /// A version field holds a value other than the GM 8.0 one.
    UnsupportedVersion { what: &'static str, found: i32 },
    /// A read ran past the end of the data.
    UnexpectedEof { wanted: usize, available: usize },
    /// A length or count field is negative.
    NegativeLength(i32),
    /// A boolean field is neither 0 nor 1.
    InvalidBool(i32),
    /// An enumerated field holds a value GM 8.0 doesn't define.
    InvalidEnum { what: &'static str, value: i32 },
    /// A zlib block failed to inflate or would inflate past [`crate::MAX_INFLATED_BLOCK`].
    Inflate(String),
    /// A compressed block has bytes left over after its last field.
    TrailingBytes(usize),
    /// An image's pixel data doesn't match its declared size.
    BadImageSize { width: u32, height: u32, len: usize },
    /// The resource tree nests deeper than [`crate::MAX_TREE_DEPTH`].
    TreeTooDeep,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at offset {}: ", self.context, self.offset)?;
        match &self.kind {
            ErrorKind::BadMagic(v) => write!(f, "not a .gmk file (magic {v})"),
            ErrorKind::UnsupportedVersion { what, found } => {
                write!(
                    f,
                    "unsupported {what} version {found} (only GM 8.0 is supported)"
                )
            }
            ErrorKind::UnexpectedEof { wanted, available } => {
                write!(
                    f,
                    "unexpected end of data (wanted {wanted} bytes, {available} left)"
                )
            }
            ErrorKind::NegativeLength(n) => write!(f, "negative length {n}"),
            ErrorKind::InvalidBool(v) => write!(f, "invalid boolean {v}"),
            ErrorKind::InvalidEnum { what, value } => write!(f, "invalid {what} {value}"),
            ErrorKind::Inflate(msg) => write!(f, "zlib: {msg}"),
            ErrorKind::TrailingBytes(n) => write!(f, "{n} trailing bytes"),
            ErrorKind::BadImageSize { width, height, len } => {
                write!(f, "{width}x{height} image has {len} bytes of pixel data")
            }
            ErrorKind::TreeTooDeep => write!(f, "resource tree is nested too deeply"),
        }
    }
}

impl std::error::Error for Error {}
