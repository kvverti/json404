use std::{
    error::Error,
    fmt::{Display, Write},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// A single punctuation character.
    Punctuation(char),
    /// The inside of a string.
    String,
    /// An escape character.
    Escape,
    /// A Unicode escape sequence.
    UnicodeEscape,
    /// A decimal digit.
    Digit,
    /// A literal JSON value.
    Literal,
}

impl Display for Expected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Expected::Punctuation(c) => f.write_char(c),
            Expected::String => f.write_str("a string component"),
            Expected::Digit => f.write_str("a digit"),
            Expected::Literal => f.write_str("a literal"),
            Expected::Escape => f.write_str("an escape character"),
            Expected::UnicodeEscape => f.write_str("a Unicode escape sequence"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    String,
    Array,
    Object,
    Number,
    Literal,
}

impl Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Reason::String => f.write_str("string"),
            Reason::Array => f.write_str("array"),
            Reason::Object => f.write_str("object"),
            Reason::Number => f.write_str("number"),
            Reason::Literal => f.write_str("literal"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxError {
    pub index: usize,
    pub reason: Reason,
    pub expected: &'static [Expected],
    pub actual: Option<u8>,
}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DisplayExpected(&'static [Expected]);
        impl Display for DisplayExpected {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let [head, tail @ ..] = &self.0 else {
                    return write!(f, "end of input");
                };
                write!(f, "{}", head)?;
                let [elems @ .., last] = tail else {
                    return Ok(());
                };
                for elem in elems {
                    write!(f, ", {}", elem)?;
                }
                write!(f, " or {}", last)
            }
        }
        write!(
            f,
            "at index {} while parsing {}: expected {}, found ",
            self.index,
            self.reason,
            DisplayExpected(self.expected)
        )?;
        match self.actual {
            Some(c) => write!(f, "'{}'", c),
            None => f.write_str("end of input"),
        }
    }
}

impl Error for SyntaxError {}

/// The parser state for our half-const JSON parser. Common functionality is
/// implemented here, variant-specific functionality is implemented in their
/// respective modules.
///
/// Currently the parser is structurally recursive, the recursion is up to
/// the number of nested objects and arrays.
#[derive(Debug)]
pub(crate) struct Parser<'src> {
    /// The source string
    src: &'src str,
    /// The position of the cursor.
    pos: usize,
    /// The position of the last matched byte. It is typically one less than the position.
    current: usize,
}

impl<'src> Parser<'src> {
    /// Constructs an initial parse state for the given source string.
    pub(crate) const fn new(src: &'src str) -> Self {
        Self {
            src,
            pos: 0,
            current: 0,
        }
    }

    /// Advances the matched portion of the source string and returns the match.
    pub(crate) const fn match_advance(&mut self) -> &'src str {
        let (slice, src) = self.src.split_at(self.pos);
        self.src = src;
        self.pos = 0;
        self.current = 0;
        slice
    }

    /// Returns the next byte of the input.
    pub(crate) const fn next(&mut self) -> Option<u8> {
        self.current = self.pos;
        if !(self.pos < self.src.len()) {
            return None;
        }
        let byte = self.src.as_bytes()[self.pos];
        self.pos += 1;
        Some(byte)
    }

    /// Returns the next non-whitespace byte of the input.
    pub(crate) const fn next_non_ws(&mut self) -> Option<u8> {
        loop {
            match self.next() {
                Some(b' ' | b'\t' | b'\n' | b'\r') => continue,
                ret => break ret,
            }
        }
    }

    pub(crate) const fn error(&self, reason: Reason, expected: &'static [Expected]) -> SyntaxError {
        SyntaxError {
            index: self.current,
            reason,
            expected,
            actual: if self.current < self.src.len() {
                Some(self.src.as_bytes()[self.current])
            } else {
                None
            },
        }
    }
}

macro_rules! try_parse {
    ($res:expr) => {
        match $res {
            Some(Ok(v)) => Some(v),
            Some(Err(e)) => return Some(Err(e)),
            None => None,
        }
    };
}
pub(crate) use try_parse;
