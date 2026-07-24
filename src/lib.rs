//! JSON404 is a fully ECMA-404 compliant JSON parser and generator.

use std::{fmt::Display, ops::Deref};

pub mod array;
pub mod object;
pub mod number;
pub mod string;
pub mod parse;

/// A specialization of [`std::borrow::Cow`] to slices. This type is covariant in `T`
/// while Cow is invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CowSlice<'src, T> {
    Borrowed(&'src [T]),
    Owned(Vec<T>),
}

impl<T> CowSlice<'_, T> {
    const fn as_slice(&self) -> &[T] {
        match self {
            CowSlice::Borrowed(values) => values,
            CowSlice::Owned(values) => values.as_slice(),
        }
    }
}

impl<T: Clone> CowSlice<'_, T> {
    fn to_mut(&mut self) -> &mut Vec<T> {
        if let Self::Borrowed(items) = self {
            *self = CowSlice::Owned(items.to_owned());
        }
        let Self::Owned(items) = self else {
            unreachable!();
        };
        items
    }

    fn into_owned(self) -> Vec<T> {
        match self {
            CowSlice::Borrowed(items) => items.to_vec(),
            CowSlice::Owned(items) => items,
        }
    }
} 

impl<T> Deref for CowSlice<'_, T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value<'src> {
    Array(array::Array<'src>),
    Obeject(object::Object<'src>),
    Number(number::Number<'src>),
    String(string::String<'src>),
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
