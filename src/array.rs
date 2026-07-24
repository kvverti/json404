use std::{borrow::Borrow, fmt::{Debug, Display, Write}, ops::Deref};

use crate::{CowSlice, Value};

/// A JSON array, composed of a sequence of JSON values. This type dereferences
/// to a slice of values, and can be easily converted to and from a slice or `Vec`.
/// Like all JSON types, arrays are clone-on-write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array<'src> {
    elements: CowSlice<'src, Value<'src>>,
}

impl<'src> Array<'src> {
    /// Constructs a new empty array.
    pub const fn new() -> Self {
        Self {
            elements: CowSlice::Borrowed(&[]),
        }
    }

    /// Constructs an array containing the given values. The slice is used
    /// as the backing storage for this array.
    pub const fn from_slice(values: &'src [Value<'src>]) -> Self {
        Self {
            elements: CowSlice::Borrowed(values)
        }
    }

    /// Produce an array that borrows from this array. It is generally better to store a
    /// borrowed array than a reference to an array.
    pub const fn borrowed(&self) -> Array<'_> {
        Array {
            elements: CowSlice::Borrowed(self.elements.as_slice()),
        }
    }

    /// Produce an array that contains clones of any borrowed data. This performs a
    /// "deep clone", as opposed to the "shallow clone" performed by the various clone-on-write
    /// methods.
    pub fn into_owned(self) -> Array<'static> {
        Array {
            elements: CowSlice::Owned(
                self.elements
                    .into_owned()
                    .into_iter()
                    .map(Value::into_owned)
                    .collect(),
            ),
        }
    }

    /// Get the slice of values underlying this array.
    pub const fn as_slice(&self) -> &[Value<'src>] {
        self.elements.as_slice()
    }

    /// Get a mutable slice of this array's values, cloning if necessary.
    pub fn as_mut_slice(&mut self) -> &mut [Value<'src>] {
        self.elements.to_mut().as_mut_slice()
    }

    /// Unwrap this array into an owned `Vec`, cloning if necessary.
    pub fn to_vec(self) -> Vec<Value<'src>> {
        self.elements.into_owned()
    }

    /// Get a mutable reference to this array's values as a `Vec`, cloning
    /// if necessary.
    pub fn to_mut_vec(&mut self) -> &mut Vec<Value<'src>> {
        self.elements.to_mut()
    }
}

impl Default for Array<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'src> Deref for Array<'src> {
    type Target = [Value<'src>];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<'src> Borrow<[Value<'src>]> for Array<'src> {
    fn borrow(&self) -> &[Value<'src>] {
        self.as_slice()
    }
}

impl<'src> From<&'src [Value<'src>]> for Array<'src> {
    fn from(value: &'src [Value<'src>]) -> Self {
        Self::from_slice(value)
    }
}

impl<'src> From<Vec<Value<'src>>> for Array<'src> {
    fn from(value: Vec<Value<'src>>) -> Self {
        Self {
            elements: CowSlice::Owned(value)
        }
    }
}

impl<'src> FromIterator<Value<'src>> for Array<'src> {
    fn from_iter<T: IntoIterator<Item = Value<'src>>>(iter: T) -> Self {
        Self {
            elements: CowSlice::Owned(Vec::from_iter(iter)),
        }
    }
}

impl<'src> PartialEq<[Value<'src>]> for Array<'src> {
    fn eq(&self, other: &[Value<'src>]) -> bool {
        self.as_slice() == other
    }
}

impl Display for Array<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('[')?;
        if let [first, rest @ ..] = &*self.elements {
            Display::fmt(first, f)?;
            for value in rest {
                write!(f, ", {value}")?;
            }
        }
        f.write_char(']')
    }
}
