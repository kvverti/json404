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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'src> {
    Array(Array<'src>),
    Obeject(Object<'src>),
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
            Value::Obeject(object) => Value::Obeject(object.borrowed()),
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
            Value::Obeject(object) => Value::Obeject(object.into_owned()),
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
            Value::Obeject(object) => Display::fmt(object, f),
            Value::Number(number) => Display::fmt(number, f),
            Value::String(string) => Display::fmt(string, f),
            Value::True => f.write_str("true"),
            Value::False => f.write_str("false"),
            Value::Null => f.write_str("null"),
        }
    }
}
