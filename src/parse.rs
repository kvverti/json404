use crate::error::{Expected, Reason, SyntaxError};

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
