use crate::{BuildError, Key};
use fst::{
    map::{Keys, Stream, Values},
    raw::Output,
    Map, MapBuilder, Streamer,
};
use h3o::CellIndex;
use std::io;

/// A read-only map of H3 cell indexes.
pub struct FrozenMap<D>(Map<D>);

impl<D: AsRef<[u8]>> FrozenMap<D> {
    /// Creates a map from its representation as a raw byte sequence.
    ///
    /// This accepts anything that can be cheaply converted to a `&[u8]`. The
    /// caller is responsible for guaranteeing that the given bytes refer to
    /// a valid map. While memory safety will not be violated by invalid input,
    /// a panic could occur while reading the map at any point.
    ///
    /// # Errors
    ///
    /// The mapmust have been written with a compatible builder. If the format
    /// is invalid or if there is a mismatch between the API version of this
    /// library and the map, then an error is returned.
    pub fn new(data: D) -> Result<Self, BuildError> {
        Ok(Map::new(data).map(Self)?)
    }

    /// Returns the number of elements in this map.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if and only if this map is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Tests the membership of a single H3 cell index.
    ///
    /// Returns true if the cell index or one of its ancestor is present.
    pub fn contains_key(&self, index: CellIndex) -> Option<CellIndex> {
        let fst = self.0.as_fst();
        let key = Key::from(index);

        let mut node = fst.root();
        for (i, b) in key.as_ref().iter().enumerate() {
            let idx = node.find_input(*b)?;
            node = fst.node(node.transition_addr(idx));
            if node.is_final() {
                return Some(Key::from(&key.as_ref()[..=i]).into());
            }
        }
        None
    }

    /// Retrieves the value associated with a cell index.
    ///
    /// If the cell index and none of its ancestor exist, then `None` is
    /// returned.
    pub fn get(&self, index: CellIndex) -> Option<(CellIndex, u64)> {
        let fst = self.0.as_fst();
        let key = Key::from(index);
        let mut output = Output::zero();

        let mut node = fst.root();
        for (i, b) in key.as_ref().iter().enumerate() {
            let idx = node.find_input(*b)?;
            let transition = node.transition(idx);
            output = output.cat(transition.out);
            node = fst.node(transition.addr);
            if node.is_final() {
                return Some((
                    Key::from(&key.as_ref()[..=i]).into(),
                    output.value(),
                ));
            }
        }
        None
    }

    /// Return a lexicographically ordered stream of all key-value pairs in this
    /// map.
    pub fn iter(&self) -> FrozenMapIterator<'_> {
        FrozenMapIterator::new(self)
    }

    /// Return a lexicographically ordered stream of all cells in this map.
    pub fn keys(&self) -> FrozenMapKeys<'_> {
        FrozenMapKeys::new(self)
    }

    /// Return a stream of all values in this map ordered lexicographically by
    /// each value's corresponding key.
    pub fn values(&self) -> FrozenMapValues<'_> {
        FrozenMapValues::new(self)
    }
}

impl FrozenMap<Vec<u8>> {
    /// Create a `FrozenMap` from an iterator of ordered H3 cell indexes and
    /// associated values.
    ///
    /// Note that this is a convenience function to build a map in memory.
    /// To build a map that streams to an arbitrary `io::Write`, use
    /// `FrozenMapBuilder`.
    ///
    /// # Errors
    ///
    /// If the iterator does not yield unique indexes in lexicographic order,
    /// then an error is returned.
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = (CellIndex, u64)>,
    ) -> Result<Self, BuildError> {
        let mut builder = FrozenMapBuilder::memory();
        builder.extend_iter(iter)?;
        Self::new(builder.into_inner()?)
    }

    /// Returns the binary contents of this map.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_fst().as_bytes()
    }
}

// ------------------------------------------------------------------------------

/// A builder for creating a frozen map.
pub struct FrozenMapBuilder<W>(MapBuilder<W>);

impl<W: io::Write> FrozenMapBuilder<W> {
    /// Create a builder that builds a map by writing it to `wtr` in a
    /// streaming fashion.
    ///
    /// # Errors
    ///
    /// If there was a problem writing to the underlying writer, an error is
    /// returned.
    pub fn new(wtr: W) -> Result<Self, BuildError> {
        MapBuilder::new(wtr).map(Self).map_err(Into::into)
    }

    /// Insert a new key-value pair into the map.
    ///
    ///# Errors
    ///
    /// If a cell index is inserted that is less than any previous cell index
    /// added, then an error is returned. Similarly, if there was a problem
    /// writing to the underlying writer, an error is returned.
    pub fn insert(
        &mut self,
        index: CellIndex,
        value: u64,
    ) -> Result<(), BuildError> {
        self.0.insert(Key::from(index), value).map_err(Into::into)
    }

    /// Calls insert on each cell index in the iterator.
    ///
    /// If an error occurred while adding an element, processing is stopped
    /// and the error is returned.
    ///
    /// # Errors
    ///
    /// If an error occurred while adding an element, processing is stopped
    /// and the error is returned.
    pub fn extend_iter(
        &mut self,
        iter: impl IntoIterator<Item = (CellIndex, u64)>,
    ) -> Result<(), BuildError> {
        self.0
            .extend_iter(
                iter.into_iter()
                    .map(|(index, value)| (Key::from(index), value)),
            )
            .map_err(Into::into)
    }

    /// Finishes the construction of the map and flushes the underlying
    /// writer. After completion, the data written to `W` may be read using
    /// one of `FrozenMap`'s constructor methods.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem writing to the underlying
    /// writer.
    pub fn finish(self) -> Result<(), BuildError> {
        self.0.finish().map_err(Into::into)
    }

    /// Just like `finish`, except it returns the underlying writer after
    /// flushing it.
    ///
    /// # Errors
    ///
    /// Returns an error if there was a problem writing to the underlying
    /// writer.
    pub fn into_inner(self) -> Result<W, BuildError> {
        self.0.into_inner().map_err(Into::into)
    }
}

impl FrozenMapBuilder<Vec<u8>> {
    /// Create a builder that builds a map in memory.
    #[must_use]
    #[inline]
    pub fn memory() -> Self {
        Self(MapBuilder::memory())
    }

    /// Finishes the construction of the map and returns it.
    #[must_use]
    #[inline]
    pub fn into_map(self) -> FrozenMap<Vec<u8>> {
        FrozenMap(self.0.into_map())
    }
}

// ------------------------------------------------------------------------------

/// An iterator over the key-value pair of a `FrozenMap`.
pub struct FrozenMapIterator<'a> {
    stream: Stream<'a>,
    len: usize,
    count: usize,
}

impl<'a> FrozenMapIterator<'a> {
    fn new<D>(map: &'a FrozenMap<D>) -> Self
    where
        D: AsRef<[u8]>,
    {
        Self {
            stream: map.0.stream(),
            len: map.len(),
            count: 0,
        }
    }
}

impl Iterator for FrozenMapIterator<'_> {
    type Item = (CellIndex, u64);

    fn next(&mut self) -> Option<Self::Item> {
        self.stream.next().map(|(key, value)| {
            self.count += 1;
            (Key::from(key).into(), value)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for FrozenMapIterator<'_> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.len - self.count
    }
}

// ------------------------------------------------------------------------------

/// An iterator over the keys of a `FrozenMap`.
pub struct FrozenMapKeys<'a> {
    keys: Keys<'a>,
    len: usize,
    count: usize,
}

impl<'a> FrozenMapKeys<'a> {
    fn new<D>(map: &'a FrozenMap<D>) -> Self
    where
        D: AsRef<[u8]>,
    {
        Self {
            keys: map.0.keys(),
            len: map.len(),
            count: 0,
        }
    }
}

impl Iterator for FrozenMapKeys<'_> {
    type Item = CellIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.keys.next().map(|key| {
            self.count += 1;
            Key::from(key).into()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for FrozenMapKeys<'_> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.len - self.count
    }
}

// ------------------------------------------------------------------------------

/// An iterator over the values of a `FrozenMap`.
pub struct FrozenMapValues<'a> {
    values: Values<'a>,
    len: usize,
    count: usize,
}

impl<'a> FrozenMapValues<'a> {
    fn new<D>(map: &'a FrozenMap<D>) -> Self
    where
        D: AsRef<[u8]>,
    {
        Self {
            values: map.0.values(),
            len: map.len(),
            count: 0,
        }
    }
}

impl Iterator for FrozenMapValues<'_> {
    type Item = u64;

    fn next(&mut self) -> Option<Self::Item> {
        self.values.next().map(|value| {
            self.count += 1;
            value
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for FrozenMapValues<'_> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.len - self.count
    }
}
