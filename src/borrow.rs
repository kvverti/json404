use std::{borrow::Cow, ops::Deref};

pub(crate) const fn as_str<'s>(cow: &'s Cow<'_, str>) -> &'s str {
    match cow {
        Cow::Borrowed(s) => s,
        Cow::Owned(s) => s.as_str(),
    }
}

/// A specialization of [`std::borrow::Cow`] to slices. This type is covariant in `T`
/// while Cow is invariant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CowSlice<'src, T> {
    Borrowed(&'src [T]),
    Owned(Vec<T>),
}

impl<T> CowSlice<'_, T> {
    pub(crate) const fn as_slice(&self) -> &[T] {
        match self {
            CowSlice::Borrowed(values) => values,
            CowSlice::Owned(values) => values.as_slice(),
        }
    }
}

impl<T: Clone> CowSlice<'_, T> {
    pub(crate) fn to_mut(&mut self) -> &mut Vec<T> {
        if let Self::Borrowed(items) = self {
            *self = CowSlice::Owned(items.to_owned());
        }
        let Self::Owned(items) = self else {
            unreachable!();
        };
        items
    }

    pub(crate) fn into_owned(self) -> Vec<T> {
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