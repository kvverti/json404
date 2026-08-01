use std::{borrow::Cow, fmt::Display, mem, num::NonZeroUsize};

use crate::error::{Expected, Reason, SyntaxError};

#[derive(Debug, Clone, Eq)]
pub struct Number<'src> {
    // SAFETY: the sole constructor, `self::parse`, properly computes relevant indicies.
    /// The full source JSON representation of the number.
    source: Cow<'src, str>,
    /// The byte index after the end of the integer part of the number.
    int_end: NonZeroUsize,
    /// The byte index after the decimal part of the number.
    dec_end: Option<NonZeroUsize>,
    /// Whether the number is negative.
    negative: bool,
    /// The case of the exponent part. None if there is no exponent part.
    exp_case: Option<Case>,
    /// The sign of the exponent part, if present.
    exp_sign: Option<Sign>,
}

impl Number<'_> {
    pub fn borrowed(&self) -> Number<'_> {
        Number {
            source: Cow::Borrowed(&self.source),
            ..*self
        }
    }

    pub fn into_owned(self) -> Number<'static> {
        Number {
            source: Cow::Owned(self.source.into_owned()),
            ..self
        }
    }

    /// The JSON text this Number represents. This representation may not be able to be
    /// directly parsed as an integer or floating-point number.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Whether this JSON number is negative.
    pub fn is_negative(&self) -> bool {
        self.negative
    }

    /// Whether this JSON number has only a (possibly negative) integer component.
    /// This method does not consider whether the value of the number can be interpeted as a floating-point
    /// or decimal integer.
    pub fn is_integer(&self) -> bool {
        self.dec_end.is_none() && self.exp_case.is_none()
    }

    /// The integer component of this number as a sequence of digits.
    pub fn integer_digits(&self) -> &[Digit] {
        let start_idx = self.negative.into();
        // SAFETY: start_idx is the position after any negative sign
        unsafe { make_digits(self.source[start_idx..self.int_end.get()].as_bytes()) }
    }

    /// The decimal component of this numberas a sequence of digits. If there is no decimal component, returns an empty slice.
    pub fn decimal_digits(&self) -> &[Digit] {
        let start_idx = self.int_end.get() + 1;
        let Some(end_idx) = self.dec_end else {
            return &[];
        };
        // SAFETY:
        // - the decimal is present
        // - start_idx is the position after the decimal point `.`
        unsafe { make_digits(self.source[start_idx..end_idx.get()].as_bytes()) }
    }

    /// The exponent component of this number.
    pub fn exponent(&self) -> Option<Exponent<'_>> {
        let case = self.exp_case?;
        // safety invariant: start_idx is either the end of the decimal part, or if no decimal then the end of the integer part,
        // plus one (for the e/E), plus one for the sign if a sign is present
        let start_idx =
            self.dec_end.unwrap_or(self.int_end).get() + 1 + usize::from(self.exp_sign.is_some());
        Some(Exponent {
            exp_digits: Cow::Borrowed(&self.source[start_idx..]),
            case,
            sign: self.exp_sign,
        })
    }

    pub fn to_u8(&self) -> Option<u8> {
        self.source[..self.int_end.get()].parse().ok()
    }

    pub fn to_u16(&self) -> Option<u8> {
        self.source[..self.int_end.get()].parse().ok()
    }

    pub fn to_u32(&self) -> Option<u8> {
        self.source[..self.int_end.get()].parse().ok()
    }

    pub fn to_u64(&self) -> Option<u8> {
        self.source[..self.int_end.get()].parse().ok()
    }

    pub fn to_u128(&self) -> Option<u128> {
        self.source[..self.int_end.get()].parse().ok()
    }

    pub fn to_i128(&self) -> Option<i128> {
        self.source[..self.int_end.get()].parse().ok()
    }

    pub fn to_f64(&self) -> Option<f64> {
        todo!()
    }
}

impl PartialEq for Number<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source
    }
}

impl Display for Number<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.source)
    }
}

impl TryFrom<String> for Number<'static> {
    type Error = SyntaxError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let Number {
            source: _,
            int_end,
            dec_end,
            negative,
            exp_case,
            exp_sign,
        } = parse(&value, 0)?.1;
        Ok(Number {
            source: Cow::Owned(value),
            int_end,
            dec_end,
            negative,
            exp_case,
            exp_sign,
        })
    }
}

impl<'src> TryFrom<&'src str> for Number<'src> {
    type Error = SyntaxError;

    fn try_from(value: &'src str) -> Result<Self, Self::Error> {
        Ok(parse(value, 0)?.1)
    }
}

macro_rules! from_impls {
    ($($type:ty)*) => {
        $(
            impl From<$type> for Number<'static> {
                fn from(value: $type) -> Self {
                    value.to_string().try_into().expect(concat!(stringify!($type), " must be stringifed into JSON-compatible syntax"))
                }
            }
        )*
    };
}
from_impls! {
    u8 u16 u32 u64 u128
    i8 i16 i32 i64 i128
    f32 f64
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exponent<'src> {
    exp_digits: Cow<'src, str>,
    case: Case,
    sign: Option<Sign>,
}

impl Exponent<'_> {
    pub fn borrowed(&self) -> Exponent<'_> {
        Exponent {
            exp_digits: Cow::Borrowed(&self.exp_digits),
            ..*self
        }
    }

    pub fn into_owned(self) -> Exponent<'static> {
        Exponent {
            exp_digits: Cow::Owned(self.exp_digits.into_owned()),
            ..self
        }
    }

    pub fn case(&self) -> Case {
        self.case
    }

    pub fn sign(&self) -> Option<Sign> {
        self.sign
    }

    pub fn digits(&self) -> &[Digit] {
        // SAFETY: `Number::decimal_digits` creates a valid byte slice
        unsafe { make_digits(self.exp_digits.as_bytes()) }
    }
}

/// Cast an array of bytes into an array of digits.
/// This relies on the representation of [`Digit`] being transparent over the
/// underlying byte value.
///
/// # Safety
/// The caller must ensure that the given bytes are in the range `b'0'..=b'9'`.
unsafe fn make_digits(bytes: &[u8]) -> &[Digit] {
    debug_assert!(
        bytes.iter().all(|&b| Digit::try_from(b).is_ok()),
        "Safety: found non-digit bytes"
    );
    // SAFETY: precondition
    unsafe { mem::transmute(bytes) }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Sign {
    Positive = b'+',
    Negative = b'-',
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Case {
    Majuscule = b'E',
    Minuscule = b'e',
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)] // note: relied upon for safety
pub enum Digit {
    Zero = b'0',
    One = b'1',
    Two = b'2',
    Three = b'3',
    Four = b'4',
    Five = b'5',
    Six = b'6',
    Seven = b'7',
    Eight = b'8',
    Nine = b'9',
}

impl Digit {
    pub fn value(self) -> u8 {
        self as u8 - b'0'
    }
}

impl From<Digit> for u8 {
    fn from(value: Digit) -> Self {
        value as u8
    }
}

#[derive(Debug)]
pub struct TryFromDigitError {
    _priv: (),
}

impl TryFrom<u8> for Digit {
    type Error = TryFromDigitError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            b'0' => Ok(Digit::Zero),
            b'1' => Ok(Digit::One),
            b'2' => Ok(Digit::Two),
            b'3' => Ok(Digit::Three),
            b'4' => Ok(Digit::Four),
            b'5' => Ok(Digit::Five),
            b'6' => Ok(Digit::Six),
            b'7' => Ok(Digit::Seven),
            b'8' => Ok(Digit::Eight),
            b'9' => Ok(Digit::Nine),
            _ => Err(TryFromDigitError { _priv: () }),
        }
    }
}

pub(crate) fn parse(src: &str, cur_idx: usize) -> Result<(usize, Number<'_>), SyntaxError> {
    let negative;
    let int_end;
    let dec_end;
    let exp_case;
    let exp_sign;
    let parse_end;

    let bytes = src.as_bytes();
    let mut idx = 0;
    // sign
    negative = matches!(bytes.get(idx), Some(b'-'));
    if negative {
        idx += 1;
    }
    // first digit
    match bytes.get(idx) {
        Some(b'1'..=b'9') => {
            // consume continuation digits
            int_end = bytes[idx..]
                .iter()
                .position(|b| !matches!(b, b'0'..=b'9'))
                .map(|i| idx + i)
                .unwrap_or(bytes.len());
            idx = int_end;
        }
        Some(b'0') => {
            idx += 1;
            int_end = idx;
        }
        item => {
            return Err(SyntaxError {
                index: cur_idx + idx,
                reason: Reason::Number,
                expected: if negative {
                    &[Expected::Digit]
                } else {
                    &[Expected::Punctuation('-'), Expected::Digit]
                },
                actual: item.map(|&x| x.into()),
            });
        }
    }

    // decimal portion
    if let Some(b'.') = bytes.get(idx) {
        idx += 1;
        // check for empty decimal portion
        let Some(b'0'..=b'9') = bytes.get(idx) else {
            return Err(SyntaxError {
                index: cur_idx + idx,
                reason: Reason::Number,
                expected: &[Expected::Digit],
                actual: bytes.get(idx).map(|&c| c.into()),
            });
        };
        // consume remaining decimal digits
        dec_end = bytes[idx..]
            .iter()
            .position(|b| !matches!(b, b'0'..=b'9'))
            .map(|i| idx + i)
            .unwrap_or(bytes.len());
        idx = dec_end;
    } else {
        dec_end = 0;
    }

    // exponent portion
    if let Some(exp_sigil @ (b'e' | b'E')) = bytes.get(idx) {
        exp_case = match exp_sigil {
            b'e' => Some(Case::Minuscule),
            b'E' => Some(Case::Majuscule),
            _ => unreachable!(),
        };
        idx += 1;
        // exponent sign
        match bytes.get(idx) {
            Some(b'+') => exp_sign = Some(Sign::Positive),
            Some(b'-') => exp_sign = Some(Sign::Negative),
            Some(b'0'..=b'9') => exp_sign = None,
            next => {
                return Err(SyntaxError {
                    index: cur_idx + idx,
                    reason: Reason::Number,
                    expected: &[
                        Expected::Punctuation('+'),
                        Expected::Punctuation('-'),
                        Expected::Digit,
                    ],
                    actual: next.map(|&x| x.into()),
                });
            }
        }
        idx += 1;
        parse_end = bytes[idx..]
            .iter()
            .position(|c| !matches!(c, b'0'..=b'9'))
            .map(|i| idx + i)
            .unwrap_or(bytes.len());
    } else {
        exp_case = None;
        exp_sign = None;
        parse_end = idx;
    }

    Ok((
        parse_end,
        Number {
            source: src[..parse_end].into(),
            int_end: int_end.try_into().expect("integer part must be nonempty"),
            dec_end: dec_end.try_into().ok(),
            negative,
            exp_case,
            exp_sign,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zero() {
        let input = "012";
        let expected = (
            1,
            Number {
                source: "0".into(),
                int_end: 1.try_into().unwrap(),
                dec_end: None,
                negative: false,
                exp_case: None,
                exp_sign: None,
            },
        );
        assert_eq!(parse(input, 0).unwrap(), expected);
    }

    #[test]
    fn parse_int() {
        let input = "-123";
        let expected = (
            4,
            Number {
                source: "-123".into(),
                int_end: 4.try_into().unwrap(),
                dec_end: None,
                negative: true,
                exp_case: None,
                exp_sign: None,
            },
        );
        assert_eq!(parse(input, 0).unwrap(), expected);
    }

    #[test]
    fn parse_dec() {
        let input = "0.015,";
        let expected = (
            5,
            Number {
                source: "0.015".into(),
                int_end: 1.try_into().unwrap(),
                dec_end: Some(5.try_into().unwrap()),
                negative: false,
                exp_case: None,
                exp_sign: None,
            },
        );
        assert_eq!(parse(input, 0).unwrap(), expected);
    }

    #[test]
    fn parse_exp() {
        let input = "12E+345";
        let expected = (
            7,
            Number {
                source: "12E+345".into(),
                int_end: 2.try_into().unwrap(),
                dec_end: None,
                negative: false,
                exp_case: Some(Case::Majuscule),
                exp_sign: Some(Sign::Positive),
            },
        );
        assert_eq!(parse(input, 0).unwrap(), expected);
    }

    #[test]
    fn parse_dec_exp() {
        let input = "-123.456e789]";
        let expected = (
            12,
            Number {
                source: "-123.456e789".into(),
                int_end: 4.try_into().unwrap(),
                dec_end: Some(8.try_into().unwrap()),
                negative: true,
                exp_case: Some(Case::Minuscule),
                exp_sign: None,
            },
        );
        assert_eq!(parse(input, 0).unwrap(), expected);
    }
}
