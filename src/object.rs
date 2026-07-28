use std::{
    fmt::{Display, Write},
    slice,
};

use crate::{
    string::{self, ToCodepoints},
    CowSlice, Value,
};

type KV<'src> = (string::String<'src>, Value<'src>);

mod sealed {
    pub trait Sealed {}

    impl Sealed for super::Exact<'_> {}
    impl<T: ?Sized + super::ToCodepoints> Sealed for T {}
}

/// A trait that abstracts the notion of comparing object keys.
pub trait Key: sealed::Sealed {
    /// Whether this key is equivalent to the given string.
    fn equivalant(&self, key: string::String<'_>) -> bool;

    fn to_string<'src>(&self) -> string::String<'src>
    where
        Self: 'src;
}

/// A wrapper around a string used to compare keys by exact content.
// todo: design an API
#[derive(Debug, Clone)]
pub struct Exact<'src>(string::String<'src>);

impl Key for Exact<'_> {
    fn equivalant(&self, key: string::String<'_>) -> bool {
        key == self.0
    }

    fn to_string<'src>(&self) -> string::String<'src>
    where
        Self: 'src,
    {
        self.0.clone()
    }
}

impl<T: ?Sized + ToCodepoints> Key for T {
    fn equivalant(&self, key: string::String<'_>) -> bool {
        key.codepoint_eq(self)
    }

    fn to_string<'src>(&self) -> string::String<'src>
    where
        Self: 'src,
    {
        self.collect_to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object<'src> {
    entries: CowSlice<'src, KV<'src>>,
}

impl<'src> Object<'src> {
    /// Constructs a new empty `Object`.
    pub const fn new() -> Self {
        Self {
            entries: CowSlice::Borrowed(&[]),
        }
    }

    pub const fn borrowed(&self) -> Object<'_> {
        Object {
            entries: CowSlice::Borrowed(self.entries.as_slice()),
        }
    }

    pub fn into_owned(self) -> Object<'static> {
        Object {
            entries: CowSlice::Owned(
                self.entries
                    .into_owned()
                    .into_iter()
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect(),
            ),
        }
    }

    pub fn has_key<K: Key>(&self, key: K) -> bool {
        self.entries
            .iter()
            .any(|(k, _)| key.equivalant(k.borrowed()))
    }

    pub fn keys(&self) -> Keys<'_> {
        Keys {
            iter: self.entries.iter(),
        }
    }

    pub fn key_values<K: Key>(&self, key: K) -> KeyValues<'_, K> {
        KeyValues {
            iter: self.entries.iter(),
            key,
        }
    }

    pub fn clear(&mut self) {
        self.entries.to_mut().clear();
    }

    pub fn add(&mut self, key: string::String<'src>, value: Value<'src>) {
        self.entries.to_mut().push((key, value));
    }

    pub fn remove_all(&mut self, key: string::String<'_>) {
        self.entries.to_mut().retain(|(k, _)| *k != key);
    }

    /// Obtain mutable access to the set of entries associated with the given key.
    pub fn entries<K: 'src + Key>(&mut self, key: K) -> Entries<'src, '_, K> {
        Entries {
            key,
            entries: self.entries.to_mut(),
        }
    }
}

impl Display for Object<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_char('{')?;
        if let [first, rest @ ..] = &*self.entries {
            write!(f, "{}: {}", first.0, first.1)?;
            for (key, value) in rest {
                write!(f, ", {key}: {value}")?;
            }
        }
        f.write_char('}')
    }
}

#[derive(Debug)]
pub struct Entries<'src, 'a, K: 'src> {
    key: K,
    entries: &'a mut Vec<KV<'src>>,
}

impl<'src, K: Key> Entries<'src, '_, K> {
    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut Value<'src>> {
        todo!() as std::iter::Once<_>
    }

    /// Adds the given value to the set of values associated with the key.
    pub fn add(&mut self, value: Value<'src>) {
        let k = self.key.to_string();
        self.entries.push((k, value));
    }

    /// Get the first value associated with the key.
    pub fn first(&mut self) -> Option<&mut Value<'src>> {
        self.values_mut().next()
    }

    /// Get the last value associated with the key.
    pub fn last(&mut self) -> Option<&mut Value<'src>> {
        self.values_mut().next_back()
    }

    /// Keep the first value for which the given predicate returns `true`, and remove
    /// all others.
    pub fn keep_first(&mut self, mut f: impl FnMut(&Value<'src>) -> bool) {
        let mut found = false;
        self.retain(|v| {
            !found && {
                found = f(v);
                found
            }
        });
    }

    /// Remove all values associated with the key.
    pub fn remove_all(&mut self) {
        self.drain_all().for_each(drop);
    }

    /// Keep all values that satisfy the given predicate.
    pub fn retain(&mut self, mut f: impl FnMut(&Value<'src>) -> bool) {
        self.entries
            .retain(|(k, v)| !self.key.equivalant(k.borrowed()) || f(v));
    }

    /// Produce an iterator that removes and returns all values associated with the key.
    pub fn drain_all(&mut self) -> impl Iterator<Item = Value<'src>> {
        todo!() as std::iter::Once<_>
    }
}

#[derive(Debug, Clone)]
pub struct Keys<'a> {
    iter: slice::Iter<'a, KV<'a>>,
}

impl<'a> Iterator for Keys<'a> {
    type Item = string::String<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.iter.next()?.0.borrowed())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }

    fn count(self) -> usize {
        self.iter.count()
    }
}

impl ExactSizeIterator for Keys<'_> {}

impl DoubleEndedIterator for Keys<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(self.iter.next_back()?.0.borrowed())
    }
}

#[derive(Debug, Clone)]
pub struct KeyValues<'a, K> {
    iter: slice::Iter<'a, KV<'a>>,
    key: K,
}

impl<'a, K: Key> Iterator for KeyValues<'a, K> {
    type Item = Value<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(
            self.iter
                .find(|(k, _)| self.key.equivalant(k.borrowed()))?
                .1
                .borrowed(),
        )
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
    }
}

impl<K: Key> DoubleEndedIterator for KeyValues<'_, K> {
    fn next_back(&mut self) -> Option<Self::Item> {
        Some(
            self.iter
                .rfind(|(k, _)| self.key.equivalant(k.borrowed()))?
                .1
                .borrowed(),
        )
    }
}
