use std::{borrow::Cow, fmt::{Debug, Display}, string::String as StdString};
use crate::parse::{Expected, Reason, SyntaxError, try_parse};

/// A Unicode codepoint. A `Codepoint` is like a `char`, except that surrogate
/// codepoints are allowed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Codepoint(u32);

impl Codepoint {
    /// Constructs a `Codepoint` with the given Unicode codepoint
    /// index. Returns `None` if the index is out of range.
    pub const fn new(index: u32) -> Option<Self> {
        if index <= 0x1F_FFFF {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Constructs the `Codepoint` corresponding to the given `char`.
    pub const fn from_char(c: char) -> Self {
        Self(c as u32)
    }

    /// Returns the `char` corresponding to this codepoint, if it is
    /// not a surrogate codepoint.
    pub const fn to_char(self) -> Option<char> {
        char::from_u32(self.0)
    }

    /// Constructs a `Codepoint` on the basic multilingual plane. This plane
    /// contains the surrogate codepoints.
    pub const fn from_bmp(index: u16) -> Self {
        Self(index as u32)
    }

    /// Whether this codepoint is a surrogate codepoint.
    /// Surrogate codepoints are the range U+D800 through U+DFFF.
    pub const fn is_surrogate(self) -> bool {
        matches!(self.0, 0xD800..=0xDFFF)
    }
}

impl Debug for Codepoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "U+{:X}", self.0)?;
        if let Some(c) = self.to_char() {
            write!(f, " '{}'", c)?;
        }
        Ok(())
    }
}

impl From<char> for Codepoint {
    fn from(value: char) -> Self {
        Self::from_char(value)
    }
}

/// A trait for types which may be turned into a sequence of codepoints.
pub trait ToCodepoints {
    /// The [`Iterator`] type returned by [`Self::into_codepoints`].
    type CodepointIter<'a>: Iterator<Item = Codepoint> where Self: 'a;

    /// Produce a sequence of codepoints from the given value.
    fn to_codepoints(&self) -> Self::CodepointIter<'_>;
}

#[derive(Debug, Clone)]
pub struct StrCodepoints<'a> {
    iter: std::str::Chars<'a>,
}

impl Iterator for StrCodepoints<'_> {
    type Item = Codepoint;

    fn next(&mut self) -> Option<Self::Item> {
        Some(Codepoint::from_char(self.iter.next()?))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl DoubleEndedIterator for StrCodepoints<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(Codepoint::from_char(self.iter.next_back()?))
    }
}

impl ToCodepoints for str {
    type CodepointIter<'a> = StrCodepoints<'a>;

    fn to_codepoints(&self) -> Self::CodepointIter<'_> {
        StrCodepoints { iter: self.chars() }
    }
}

impl ToCodepoints for [Codepoint] {
    type CodepointIter<'a> = std::iter::Copied<std::slice::Iter<'a, Codepoint>>;
    
    fn to_codepoints(&self) -> Self::CodepointIter<'_> {
        self.into_iter().copied()
    }
}

/// A JSON string. A string is a sequence of Unicode code points, which may contain unpaired surrogates due to
/// Unicode escape sequences.
/// 
/// # Comparison
/// This library makes the choice to compare JSON strings literally, without normalizing Unicode escapes.
/// For example, the following pairs of JSON strings are considered unequal.
/// - `"Hello"` and `"Hell\u006F"` (Unicode escapes are not decoded)
/// - `"\uabcd"` and `"\uABCD"` (case of Unicode escapes is not normalized)
/// - `"\n"` and `"\u000A` (Unicode escapes and simple escapes are not normalized)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct String<'src> {
    /// The underlying data.
    bytes: Cow<'src, str>,
}

impl<'src> String<'src> {
    pub fn borrowed(&self) -> String<'_> {
        String {
            bytes: Cow::Borrowed(&self.bytes)
        }
    }

    pub fn into_owned(self) -> String<'static> {
        String {
            bytes: Cow::Owned(self.bytes.into_owned())
        }
    }

    /// The underlying JSON string data. Escape sequences are not decoded.
    pub fn source(&self) -> &str {
        &self.bytes
    }

    /// Produce an iterator over the parts of this JSON string. Each part is one of
    /// - a `char` representing a Unicode Scalar Value other than `\`, `"`, or control characters.
    /// - a simple escape sequence representing a control character.
    /// - a Unicode escape sequence representing a Unicode codepoint from `U+0` to `U+FFFF` (including unpaired surrogates).
    pub fn parts(&self) -> Parts<'_> {
        Parts { src: self.bytes.chars() }
    }

    /// Produce an iterator over the codepoints represented in this JSON string. Surrogate pairs are treated
    /// as two separate codepoints.
    pub fn codepoints(&self) -> Codepoints<'_> {
        Codepoints { src: self.parts() }
    }

    pub fn chars(&self) -> Chars<'_> {
        Chars { src: self.parts().peekable() }
    }

    /// Returns whether this string matches the given codepoint sequence.
    pub fn codepoint_eq<I: ToCodepoints>(&self, other: &I) -> bool {
        let mut these_codepoints = self.codepoints();
        let mut those_codepoints = other.to_codepoints();
        loop {
            match (these_codepoints.next(), those_codepoints.next()) {
                (None, None) => break true,
                (Some(a), Some(b)) if a != b => break false,
                _ => continue,
            }
        }
    }
}

impl Display for String<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", &self.bytes)
    }
}

impl<'src> ToCodepoints for String<'src> {
    type CodepointIter<'a> = Codepoints<'a> where Self: 'a;

    fn to_codepoints(&self) -> Self::CodepointIter<'_> {
        self.codepoints()
    }
}

pub struct StringValueError {
    pub prefix: StdString,
    pub codepoint: u16,
}

pub struct Parts<'src> {
    src: std::str::Chars<'src>,
}

impl Iterator for Parts<'_> {
    type Item = Result<StringPart, SyntaxError>;

    fn next(&mut self) -> Option<Self::Item> {
        let found = self.src.next();
        match found {
            // end of string
            Some('"') => return None,
            // escape
            Some('\\') =>
                match self.src.next() {
                    Some('u') => {
                        let mut get_digits = || {
                            let mut digit_fn = || {
                                let found = self.src.next();
                                match found.map(|c| c.try_into()) {
                                    Some(Ok(c)) => Ok(c),
                                    _ => Err(SyntaxError {
                                        index: 0,
                                        reason: Reason::String,
                                        expected: &[Expected::UnicodeEscape],
                                        actual: found,
                                    }),
                                }
                            };
                            Ok([
                                digit_fn()?,
                                digit_fn()?,
                                digit_fn()?,
                                digit_fn()?,
                            ])
                        };
                        let digits = match get_digits() {
                            Ok(digits) => digits,
                            Err(err) => return Some(Err(err)),
                        };
                        Some(Ok(StringPart::Escape(StringEscape::Unicode(HexDigits(digits)))))
                    },
                    found => match found.map(|c| c.try_into()) {
                        Some(Ok(escape)) => Some(Ok(StringPart::Escape(StringEscape::Short(escape)))),
                        _ => Some(Err(SyntaxError { index: 0, reason: Reason::String, expected: &[Expected::Escape], actual: found })),
                    }
                },
            // illegal control character or EOI
            Some('\x00'..='\x1F') | None => Some(Err(SyntaxError { index: 0, reason: Reason::String, expected: &[Expected::String], actual: found })),
            // normal character
            Some(c) => Some(Ok(StringPart::Char(c))),
        }
    }
}

pub struct Codepoints<'src> {
    src: Parts<'src>,
}

impl Iterator for Codepoints<'_> {
    type Item = Codepoint;

    fn next(&mut self) -> Option<Self::Item> {
        let part = self.src.next()?.expect("JSON string should be valid");
        match part {
            StringPart::Char(c) => Some(c.into()),
            StringPart::Escape(StringEscape::Short(code)) => Some(Codepoint::from_bmp(code as u16)),
            StringPart::Escape(StringEscape::Unicode(digits)) => Some(Codepoint::from_bmp(digits.to_codepoint())),
        }
    }
}

pub struct Chars<'src> {
    src: std::iter::Peekable<Parts<'src>>,
}

impl Iterator for Chars<'_> {
    type Item = Result<Result<char, u16>, SyntaxError>;

    fn next(&mut self) -> Option<Self::Item> {
        let part = try_parse!(self.src.next())?;
        match part {
            StringPart::Char(c) => Some(Ok(Ok(c))),
            StringPart::Escape(StringEscape::Short(escape)) => Some(Ok(Ok(char::from(escape as u8)))),
            StringPart::Escape(StringEscape::Unicode(hi_digits)) => {
                match hi_digits.to_codepoint() {
                    hi @ 0xD800..=0xDBFF => {
                        let next_part = try_parse!(self.src.peek().copied());
                        if let Some(StringPart::Escape(StringEscape::Unicode(lo_digits))) = next_part && let lo @ 0xDC00..=0xDFFF = lo_digits.to_codepoint() {
                            _ = self.src.next();
                            let mut combined_codepoint = (u32::from(hi) & 0x3FF) << 10 | (u32::from(lo) & 0x3FF);
                            combined_codepoint += 0x10000;
                            Some(Ok(Ok(char::from_u32(combined_codepoint).expect("Parsing a guaranteed valid code point"))))
                        } else {
                            Some(Ok(Err(hi)))
                        }
                    },
                    codepoint => Some(Ok(char::from_u32(codepoint.into()).ok_or(codepoint)))
                }
            }
        }
    }
}

/// A single component of a JSON string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringPart {
    /// Any Unicode Scalar Value except for control characters, ", or \
    Char(char),
    /// An escape code.
    Escape(StringEscape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringEscape {
    Short(ShortEscape),
    Unicode(HexDigits),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ShortEscape {
    Quotation = b'"',
    ReverseSolidus = b'\\',
    Solidus = b'/',
    Backspace = b'\x08',
    FormFeed = b'\x0C',
    LineFeed = b'\n',
    CarriageReturn = b'\r',
    Tabulation = b'\t',
}

#[derive(Debug)]
pub struct ParseEscapeError;

impl TryFrom<char> for ShortEscape {
    type Error = ParseEscapeError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '"' => Ok(Self::Quotation),
            '\\' => Ok(Self::ReverseSolidus),
            '/' => Ok(Self::Solidus),
            'b' => Ok(Self::Backspace),
            'f' => Ok(Self::FormFeed),
            'n' => Ok(Self::LineFeed),
            'r' => Ok(Self::CarriageReturn),
            't' => Ok(Self::Tabulation),
            _ => Err(ParseEscapeError),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HexDigits(pub [HexDigit; 4]);

impl HexDigits {
    pub fn to_codepoint(self) -> u16 {
        let nybbles = self.0.map(|digit| u8::from(digit.value()));
        let bytes = [nybbles[0] << 4 | nybbles[1], nybbles[2] << 4 | nybbles[3]];
        u16::from_be_bytes(bytes)
    }
}

/// A hexadecimal digit.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum HexDigit {
    Eight = b'8',
    Five = b'5',
    Four = b'4',
    MajusculeA = b'A',
    MajusculeB = b'B',
    MajusculeC = b'C',
    MajusculeD = b'D',
    MajusculeE = b'E',
    MajusculeF = b'F',
    MinusculeA = b'a',
    MinusculeB = b'b',
    MinusculeC = b'c',
    MinusculeD = b'd',
    MinusculeE = b'e',
    MinusculeF = b'f',
    Nine = b'9',
    One = b'1',
    Seven = b'7',
    Six = b'6',
    Three = b'3',
    Two = b'2',
    Zero = b'0',
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum HexValue {
    Zero,
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Eleven,
    Twelve,
    Thirteen,
    Fourteen,
    Fifteen,
}

impl HexValue {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl From<HexDigit> for HexValue {
    fn from(value: HexDigit) -> Self {
        value.value()
    }
}

impl From<HexValue> for u8 {
    fn from(value: HexValue) -> Self {
        value.as_u8()
    }
}

#[derive(Debug)]
pub struct ParseHexDigitError;

impl TryFrom<char> for HexDigit {
    type Error = ParseHexDigitError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value {
            '0' => Ok(HexDigit::Zero),
            '1' => Ok(HexDigit::One),
            '2' => Ok(HexDigit::Two),
            '3' => Ok(HexDigit::Three),
            '4' => Ok(HexDigit::Four),
            '5' => Ok(HexDigit::Five),
            '6' => Ok(HexDigit::Six),
            '7' => Ok(HexDigit::Seven),
            '8' => Ok(HexDigit::Eight),
            '9' => Ok(HexDigit::Nine),
            'A' => Ok(HexDigit::MajusculeA),
            'a' => Ok(HexDigit::MinusculeA),
            'B' => Ok(HexDigit::MajusculeB),
            'b' => Ok(HexDigit::MinusculeB),
            'C' => Ok(HexDigit::MajusculeC),
            'c' => Ok(HexDigit::MinusculeC),
            'D' => Ok(HexDigit::MajusculeD),
            'd' => Ok(HexDigit::MinusculeD),
            'E' => Ok(HexDigit::MajusculeE),
            'e' => Ok(HexDigit::MinusculeE),
            'F' => Ok(HexDigit::MajusculeF),
            'f' => Ok(HexDigit::MinusculeF),
            _ => Err(ParseHexDigitError)
        }
    }
}

impl HexDigit {
    /// The numerical value of this digit.
    pub fn value(self) -> HexValue {
        match self {
            HexDigit::Eight => HexValue::Eight,
            HexDigit::Five => HexValue::Five,
            HexDigit::Four => HexValue::Four,
            HexDigit::MajusculeA => HexValue::Ten,
            HexDigit::MajusculeB => HexValue::Eleven,
            HexDigit::MajusculeC => HexValue::Twelve,
            HexDigit::MajusculeD => HexValue::Thirteen,
            HexDigit::MajusculeE => HexValue::Fourteen,
            HexDigit::MajusculeF => HexValue::Fifteen,
            HexDigit::MinusculeA => HexValue::Ten,
            HexDigit::MinusculeB => HexValue::Eleven,
            HexDigit::MinusculeC => HexValue::Twelve,
            HexDigit::MinusculeD => HexValue::Thirteen,
            HexDigit::MinusculeE => HexValue::Fourteen,
            HexDigit::MinusculeF => HexValue::Fifteen,
            HexDigit::Nine => HexValue::Nine,
            HexDigit::One => HexValue::One,
            HexDigit::Seven => HexValue::Seven,
            HexDigit::Six => HexValue::Six,
            HexDigit::Three => HexValue::Three,
            HexDigit::Two => HexValue::Two,
            HexDigit::Zero => HexValue::Zero,
        }
    }
}
