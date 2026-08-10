//! JSON404 is a fully ECMA-404 compliant JSON parser and generator.

use std::fmt::Display;

pub mod array;
pub mod number;
pub mod object;
pub mod string;

pub mod error;
pub mod parse;

mod borrow;

pub use array::Array;
pub use number::Number;
pub use object::Object;
pub use string::String;

pub type Result<T> = std::result::Result<T, error::SyntaxError>;

#[doc(hidden)]
pub use std as __std;

/// Constructs a JSON value from JSON syntax.
#[macro_export]
macro_rules! value {
    (true) => ($crate::Value::True);
    (false) => ($crate::Value::False);
    (null) => ($crate::Value::Null);
    ([$($t:tt)*]) => ($crate::Value::Array($crate::array![$($t)*]));
    ({$($t:tt)*}) => ($crate::Value::Object($crate::object!{$($t)*}));
    ($prim:literal) => (
        match $crate::__std::primitive::str::as_bytes($crate::__std::stringify!($prim)) {
            [b'-' | b'0'..=b'9', ..] => todo!(), // number
            _ => $crate::Value::String($crate::string::macro_parse($crate::__std::concat!("\"", $prim, "\"")))
        }
    );
}

/// Constructs a JSON object from a sequence of key-value pairs.
/// 
/// ```
/// const USER: json404::Object<'_> = json404::object! {
///     "name": "Vikoryn Vedtren",
///     "email": "vvedtren@example.com",
///     "admin": true
/// };
/// ```
#[macro_export]
macro_rules! object {
    ($($key:literal : $value:tt),*) => (
        $crate::Object::from_slice(const {
            &[$((
                $crate::string::macro_parse($crate::__std::concat!("\"", $key, "\"")),
                $crate::value!($value),
            )),*]
        })
    );
}

/// Constructs a JSON array from a sequence of values.
/// 
/// ```
/// const VALUES: json404::Array<'_> = json404::array![0.0, "", false, null, [], {}];
/// ```
#[macro_export]
macro_rules! array {
    ($($elem:tt),*) => ($crate::Array::from_slice(const { &[$($crate::value!($elem)),*] }));
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'src> {
    Array(Array<'src>),
    Object(Object<'src>),
    Number(Number<'src>),
    String(String<'src>),
    True,
    False,
    Null,
}

impl Value<'_> {
    pub fn borrowed(&self) -> Value<'_> {
        match self {
            Value::Array(array) => Value::Array(array.borrowed()),
            Value::Object(object) => Value::Object(object.borrowed()),
            Value::Number(number) => Value::Number(number.borrowed()),
            Value::String(string) => Value::String(string.borrowed()),
            Value::True => Value::True,
            Value::False => Value::False,
            Value::Null => Value::Null,
        }
    }

    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Array(array) => Value::Array(array.into_owned()),
            Value::Object(object) => Value::Object(object.into_owned()),
            Value::Number(number) => Value::Number(number.into_owned()),
            Value::String(string) => Value::String(string.into_owned()),
            Value::True => Value::True,
            Value::False => Value::False,
            Value::Null => Value::Null,
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    pub fn is_true(&self) -> bool {
        matches!(self, Self::True)
    }

    pub fn is_false(&self) -> bool {
        matches!(self, Self::False)
    }
}

impl Display for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Array(array) => Display::fmt(array, f),
            Value::Object(object) => Display::fmt(object, f),
            Value::Number(number) => Display::fmt(number, f),
            Value::String(string) => Display::fmt(string, f),
            Value::True => f.write_str("true"),
            Value::False => f.write_str("false"),
            Value::Null => f.write_str("null"),
        }
    }
}
