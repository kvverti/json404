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
    pub actual: Option<char>,
}

impl Display for SyntaxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct DisplayExpected(&'static [Expected]);
        impl Display for DisplayExpected {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let [head, tail @ ..] = &self.0 else {
                    return Ok(());
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
