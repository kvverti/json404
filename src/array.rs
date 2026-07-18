use std::{fmt::{Debug, Display, Write}, ops::{Index, IndexMut}};

use crate::{CowSlice, Value};

/// A JSON array, composed of a sequence of JSON values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Array<'src> {
    elements: CowSlice<'src, Value<'src>>,
}

impl<'src> Array<'src> {
    pub fn borrowed(&self) -> Array<'_> {
        Array {
            elements: CowSlice::Borrowed(&self.elements),
        }
    }

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

    pub fn get(&self, index: usize) -> Option<&Value<'src>> {
        self.elements.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value<'src>> {
        self.elements.to_mut().get_mut(index)
    }

    pub fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn as_slice(&self) -> &[Value<'src>] {
        &self.elements
    }

    pub fn as_mut_slice(&mut self) -> &mut [Value<'src>] {
        self.elements.to_mut().as_mut_slice()
    }

    pub fn push(&mut self, value: Value<'src>) {
        self.elements.to_mut().push(value)
    }
}

impl<'src> Index<usize> for Array<'src> {
    type Output = Value<'src>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.elements[index]
    }
}

impl<'src> IndexMut<usize> for Array<'src> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.elements.to_mut()[index]
    }
}

impl<'src> FromIterator<Value<'src>> for Array<'src> {
    fn from_iter<T: IntoIterator<Item = Value<'src>>>(iter: T) -> Self {
        Self {
            elements: CowSlice::Owned(Vec::from_iter(iter)),
        }
    }
}

impl<'src> Extend<Value<'src>> for Array<'src> {
    fn extend<T: IntoIterator<Item = Value<'src>>>(&mut self, iter: T) {
        self.elements.to_mut().extend(iter);
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
