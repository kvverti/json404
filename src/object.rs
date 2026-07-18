use std::{fmt::{Display, Write}, slice};

use crate::{string, CowSlice, Value};

type KV<'src> = (string::String<'src>, Value<'src>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object<'src> {
    entries: CowSlice<'src, KV<'src>>,
}

impl<'src> Object<'src> {
    pub fn borrowed(&self) -> Object<'_> {
        Object {
            entries: CowSlice::Borrowed(&self.entries),
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

    pub fn has_key(&self, key: string::String<'_>) -> bool {
        self.entries.iter().any(|(k, _)| *k == key)
    }

    pub fn keys(&self) -> Keys<'_> {
        Keys {
            iter: self.entries.iter(),
        }
    }

    pub fn key_values<'k>(&self, key: string::String<'k>) -> KeyValues<'_, 'k> {
        KeyValues {
            iter: self.entries.iter(),
            key,
        }
    }

    pub fn first(&self, key: string::String<'_>) -> Option<Value<'_>> {
        self.key_values(key).next()
    }

    pub fn last(&self, key: string::String<'_>) -> Option<Value<'_>> {
        self.key_values(key).next_back()
    }

    pub fn first_mut(&mut self, key: string::String<'_>) -> Option<&mut Value<'src>> {
        Some(&mut self.entries.to_mut().iter_mut().find(|(k, _)| *k == key)?.1)
    }

    pub fn last_mut(&mut self, key: string::String<'_>) -> Option<&mut Value<'src>> {
        Some(&mut self.entries.to_mut().iter_mut().rfind(|(k, _)| *k == key)?.1)
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
pub struct KeyValues<'a, 'k> {
    iter: slice::Iter<'a, KV<'a>>,
    key: string::String<'k>,
}

impl<'a> Iterator for KeyValues<'a, '_> {
    type Item = Value<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (key, value) = self.iter.next()?;
            if *key == self.key {
                break Some(value.borrowed());
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.iter.len()))
    }
}

impl DoubleEndedIterator for KeyValues<'_, '_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        loop {
            let (key, value) = self.iter.next_back()?;
            if *key == self.key {
                break Some(value.borrowed());
            }
        }
    }
}
