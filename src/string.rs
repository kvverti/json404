use crate::{
    borrow::as_str,
    parse::Parser,
};
use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    mem,
    string::String as StdString,
};

mod parse;

/// A Unicode codepoint. A `Codepoint` is like a `char`, except that surrogate
/// codepoints are allowed.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Codepoint(u32);

impl Codepoint {
    /// Constructs a `Codepoint` with the given Unicode codepoint
    /// index. Returns `None` if the index is out of range.
    pub const fn new(index: u32) -> Option<Self> {
        if index <= char::MAX as u32 {
            Some(Self(index))
        } else {
            None
        }
    }

    /// Returns the Unicode codepoint index of this codepoint.
    pub const fn index(self) -> u32 {
        self.0
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
    type CodepointIter<'a>: Iterator<Item = Codepoint>
    where
        Self: 'a;

    /// Produce a sequence of codepoints.
    fn to_codepoints(&self) -> Self::CodepointIter<'_>;

    /// Produce a [`String`] containing the sequence of codepoints. It should produce
    /// the same result as `self.to_codepoints().collect()`, but may be able to reuse
    /// memory. You should generally call [`String::encode`] instead of directly
    /// invoking this method.
    fn collect_to_string<'src>(&self) -> String<'src>
    where
        Self: 'src,
    {
        self.to_codepoints().collect()
    }
}

impl<T: ?Sized + ToCodepoints> ToCodepoints for &T {
    type CodepointIter<'a>
        = <T as ToCodepoints>::CodepointIter<'a>
    where
        Self: 'a;

    fn to_codepoints(&self) -> Self::CodepointIter<'_> {
        (*self).to_codepoints()
    }

    fn collect_to_string<'src>(&self) -> String<'src>
    where
        Self: 'src,
    {
        (*self).collect_to_string()
    }
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

impl ToCodepoints for &str {
    type CodepointIter<'a>
        = StrCodepoints<'a>
    where
        Self: 'a;

    fn to_codepoints(&self) -> Self::CodepointIter<'_> {
        StrCodepoints { iter: self.chars() }
    }

    fn collect_to_string<'src>(&self) -> String<'src>
    where
        Self: 'src,
    {
        if self
            .bytes()
            .any(|b| matches!(b, b'\x00'..=b'\x1F' | b'\\' | b'"'))
        {
            (*self).collect_to_string()
        } else {
            // the contents are all valid literal JSON string characters
            String {
                bytes: Cow::Borrowed(self),
            }
        }
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
    /// The underlying data, not including the delimiting quotes.
    bytes: Cow<'src, str>,
}

impl<'src> String<'src> {
    /// Creates a `String` containing the given codepoint sequence. Use this function to create
    /// a `String` from a string literal or slice.
    pub fn encode<I: 'src + ToCodepoints>(codepoints: I) -> Self {
        codepoints.collect_to_string()
    }

    /// Produce a string that borrows from this string. It is generally better
    /// to store a borrowed string than a reference to a string.
    pub const fn borrowed(&self) -> String<'_> {
        String {
            bytes: Cow::Borrowed(as_str(&self.bytes)),
        }
    }

    pub fn into_owned(self) -> String<'static> {
        String {
            bytes: Cow::Owned(self.bytes.into_owned()),
        }
    }

    /// The underlying JSON string data, without the delimiting quotes. Escape sequences are not decoded.
    pub const fn source(&self) -> &str {
        as_str(&self.bytes)
    }

    /// Produce an iterator over the parts of this JSON string. Each part is one of
    /// - a `char` representing a Unicode Scalar Value other than `\`, `"`, or control characters.
    /// - a simple escape sequence representing a control character.
    /// - a Unicode escape sequence representing a Unicode codepoint from `U+0` to `U+FFFF` (including unpaired surrogates).
    pub fn parts(&self) -> Parts<'_> {
        Parts {
            src: self.bytes.chars(),
        }
    }

    /// Produce an iterator over the codepoints represented in this JSON string. Surrogate pairs are treated
    /// as two separate codepoints.
    pub fn codepoints(&self) -> Codepoints<'_> {
        Codepoints { src: self.parts() }
    }

    /// Produce an iterator over the characters represented in this JSON string. Surrogate pairs are decoded
    /// into a single char. Lone surrogates are returned as errors.
    pub fn chars(&self) -> Chars<'_> {
        Chars {
            src: self.parts().peekable(),
        }
    }

    /// Returns whether this string matches the given codepoint sequence.
    pub fn codepoint_eq<I: ?Sized + ToCodepoints>(&self, other: &I) -> bool {
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
    type CodepointIter<'a>
        = Codepoints<'a>
    where
        Self: 'a;

    fn to_codepoints(&self) -> Self::CodepointIter<'_> {
        self.codepoints()
    }

    fn collect_to_string<'src1>(&self) -> String<'src1>
    where
        Self: 'src1,
    {
        self.clone()
    }
}

impl<'src> FromIterator<Codepoint> for String<'src> {
    fn from_iter<T: IntoIterator<Item = Codepoint>>(iter: T) -> Self {
        let mut contents = StdString::new();
        for codepoint in iter {
            if let Some(c) = codepoint.to_char()
                && c != '/'
                && let Some(esc) = SimpleEscape::encode(c)
            {
                contents.push_str(esc.escape_sequence());
            } else if let Some(c @ '\u{20}'..) = codepoint.to_char() {
                contents.push(c);
            } else {
                use std::fmt::Write as _;
                assert!(codepoint.index() <= 0xFFFF);
                write!(contents, r"\u{:04X}", codepoint.index())
                    .expect("formatting to string must not fail");
            }
        }
        Self {
            bytes: Cow::Owned(contents),
        }
    }
}

impl<'src> FromIterator<Part> for String<'src> {
    fn from_iter<T: IntoIterator<Item = Part>>(iter: T) -> Self {
        let mut contents = StdString::new();
        for part in iter {
            match part {
                Part::Char(c) => contents.push(c),
                Part::SimpleEscape(escape) => contents.push_str(escape.escape_sequence()),
                Part::UnicodeEscape(escape) => {
                    contents.push_str(r"\u");
                    for digit in escape.0 {
                        contents.push(digit.as_char());
                    }
                }
            }
        }
        Self {
            bytes: Cow::Owned(contents),
        }
    }
}

/// Parses a string from JSON string data.
#[doc(hidden)]
pub const fn parse(src: &str) -> crate::Result<String<'_>> {
    match Parser::new(src).string() {
        Ok(content) => Ok(String {
            bytes: Cow::Borrowed(content),
        }),
        Err(e) => Err(e),
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
    type Item = Part;

    fn next(&mut self) -> Option<Self::Item> {
        match self.src.next()? {
            '\\' => match self
                .src
                .next()
                .expect("string must not have a partial escape sequence")
            {
                'u' => {
                    macro_rules! four {
                        ($x:expr) => {
                            [$x, $x, $x, $x]
                        };
                    }
                    let digits = four![
                        self.src
                            .next()
                            .and_then(HexDigit::from_char)
                            .expect("string must have valid Unicode escape")
                    ];
                    Some(Part::UnicodeEscape(UnicodeEscape(digits)))
                }
                c => Some(Part::SimpleEscape(
                    SimpleEscape::from_code(c)
                        .expect("string must have valid simple escape sequence"),
                )),
            },
            c => Some(Part::Char(c)),
        }
    }
}

pub struct Codepoints<'src> {
    src: Parts<'src>,
}

impl Iterator for Codepoints<'_> {
    type Item = Codepoint;

    fn next(&mut self) -> Option<Self::Item> {
        match self.src.next()? {
            Part::Char(c) => Some(c.into()),
            Part::SimpleEscape(escape) => Some(Codepoint::from_char(escape.decode())),
            Part::UnicodeEscape(escape) => Some(Codepoint::from_bmp(escape.to_codepoint())),
        }
    }
}

pub struct Chars<'src> {
    src: std::iter::Peekable<Parts<'src>>,
}

impl Iterator for Chars<'_> {
    type Item = Result<char, u16>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.src.next()? {
            Part::Char(c) => Some(Ok(c)),
            Part::SimpleEscape(escape) => Some(Ok(escape.decode())),
            Part::UnicodeEscape(hi_digits) => match hi_digits.to_codepoint() {
                // high surrogate
                hi @ 0xD800..=0xDBFF => {
                    let next_part = self.src.peek().copied();
                    if let Some(Part::UnicodeEscape(lo_digits)) = next_part
                        && let lo @ 0xDC00..=0xDFFF = lo_digits.to_codepoint()
                    {
                        _ = self.src.next();
                        let mut combined_codepoint =
                            (u32::from(hi) & 0x3FF) << 10 | (u32::from(lo) & 0x3FF);
                        combined_codepoint += 0x10000;
                        Some(Ok(char::from_u32(combined_codepoint)
                            .expect("parsing a guaranteed valid code point")))
                    } else {
                        Some(Err(hi))
                    }
                }
                // USV or low surrogate
                codepoint => Some(char::from_u32(codepoint.into()).ok_or(codepoint)),
            },
        }
    }
}

/// A single component of a JSON string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// Any Unicode Scalar Value except for control characters, ", or \
    Char(char),
    /// A simple escape sequence.
    SimpleEscape(SimpleEscape),
    /// A Unicode escape sequence.
    UnicodeEscape(UnicodeEscape),
}

/// A simple escape sequence representing a selected ASCII control code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SimpleEscape {
    Quotation = b'"',
    ReverseSolidus = b'\\',
    Solidus = b'/',
    Backspace = b'b',
    FormFeed = b'f',
    LineFeed = b'n',
    CarriageReturn = b'r',
    Tabulation = b't',
}

impl SimpleEscape {
    /// Encodes a `char` into a simple escape.
    pub const fn encode(c: char) -> Option<SimpleEscape> {
        match c {
            '"' => Some(Self::Quotation),
            '\\' => Some(Self::ReverseSolidus),
            '/' => Some(Self::Solidus),
            '\x08' => Some(Self::Backspace),
            '\x0C' => Some(Self::FormFeed),
            '\n' => Some(Self::LineFeed),
            '\r' => Some(Self::CarriageReturn),
            '\t' => Some(Self::Tabulation),
            _ => None,
        }
    }

    /// Decodes this escape code into the corresponding ASCII character.
    pub const fn decode(self) -> char {
        match self {
            Self::Quotation => '"',
            Self::ReverseSolidus => '\\',
            Self::Solidus => '/',
            Self::Backspace => '\x08',
            Self::FormFeed => '\x0C',
            Self::LineFeed => '\n',
            Self::CarriageReturn => '\r',
            Self::Tabulation => '\t',
        }
    }

    /// Returns the escape corresponding to the given code character.
    pub const fn from_code(code: char) -> Option<Self> {
        match code {
            '"' => Some(Self::Quotation),
            '\\' => Some(Self::ReverseSolidus),
            '/' => Some(Self::Solidus),
            'b' => Some(Self::Backspace),
            'f' => Some(Self::FormFeed),
            'n' => Some(Self::LineFeed),
            'r' => Some(Self::CarriageReturn),
            't' => Some(Self::Tabulation),
            _ => None,
        }
    }

    /// The JSON escape sequence that corresponds to this escape code.
    pub const fn escape_sequence(self) -> &'static str {
        match self {
            Self::Quotation => r#"\""#,
            Self::ReverseSolidus => r"\\",
            Self::Solidus => r"\/",
            Self::Backspace => r"\b",
            Self::FormFeed => r"\f",
            Self::LineFeed => r"\n",
            Self::CarriageReturn => r"\r",
            Self::Tabulation => r"\t",
        }
    }
}

impl Display for SimpleEscape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.escape_sequence())
    }
}

/// A Unicode escape sequence, represented by four hexadecimal digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UnicodeEscape(pub [HexDigit; 4]);

impl UnicodeEscape {
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

impl HexDigit {
    pub const fn from_char(c: char) -> Option<Self> {
        match c {
            '0'..='9' | 'A'..='F' | 'a'..='f' => {
                // SAFETY: these are exactly the allowed values of HexDigit
                unsafe { Some(mem::transmute::<u8, Self>(c as u8)) }
            }
            _ => None,
        }
    }

    pub fn as_char(self) -> char {
        char::from(self as u8)
    }
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
        Self::from_char(value).ok_or(ParseHexDigitError)
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
