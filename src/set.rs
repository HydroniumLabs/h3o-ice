use crate::{BuildError, Key};
use fst::{set::Stream, Set, SetBuilder, Streamer};
use h3o::{CellIndex, Resolution};
use std::io;

/// A read-only set of H3 cell indexes.
pub struct FrozenSet<D>(Set<D>);

impl<D: AsRef<[u8]>> FrozenSet<D> {
    /// Creates a set from its representation as a raw byte sequence.
    ///
    /// This accepts anything that can be cheaply converted to a `&[u8]`. The
    /// caller is responsible for guaranteeing that the given bytes refer to
    /// a valid set. While memory safety will not be violated by invalid input,
    /// a panic could occur while reading the set at any point.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use h3o_ice::FrozenSet;
    ///
    /// let bytes = [
    ///     // ...
    /// ];
    ///
    /// let set = FrozenSet::new(bytes).expect("valid set");
    /// ```
    pub fn new(data: D) -> Result<Self, BuildError> {
        Ok(Set::new(data).map(Self)?)
    }

    /// Returns the number of elements in this set.
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if and only if this set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn contains(&self, index: CellIndex) -> Option<CellIndex> {
        Resolution::range(Resolution::Zero, index.resolution())
            .rev()
            .map(|res| index.parent(res).expect("valid res"))
            .find(|&index| self.0.contains(Key::from(index)))
    }

    pub fn iter(&self) -> FrozenSetIterator<'_> {
        FrozenSetIterator::new(self)
    }
}

impl FrozenSet<Vec<u8>> {
    /// Create a `FrozenSet` from an iterator of ordered H3 cell indexes.
    ///
    /// If the iterator does not yield values in lexicographic order, then an
    /// error is returned.
    ///
    /// Note that this is a convenience function to build a set in memory.
    /// To build a set that streams to an arbitrary `io::Write`, use
    /// `FrozenSetBuilder`.
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = CellIndex>,
    ) -> Result<FrozenSet<Vec<u8>>, BuildError> {
        let mut builder = FrozenSetBuilder::memory();
        builder.extend_iter(iter)?;
        FrozenSet::new(builder.into_inner()?)
    }

    /// Returns the binary contents of this set.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_fst().as_bytes()
    }
}

// ------------------------------------------------------------------------------

pub struct FrozenSetBuilder<W>(SetBuilder<W>);

impl<W: io::Write> FrozenSetBuilder<W> {
    /// Create a builder that builds a set by writing it to `wtr` in a
    /// streaming fashion.
    pub fn new(wtr: W) -> Result<FrozenSetBuilder<W>, BuildError> {
        SetBuilder::new(wtr).map(Self).map_err(Into::into)
    }

    /// Insert a new cell index into the set.
    ///
    /// If a cell index is inserted that is less than any previous cell index
    /// added, then an error is returned. Similarly, if there was a problem
    /// writing to the underlying writer, an error is returned.
    pub fn insert(&mut self, index: CellIndex) -> Result<(), BuildError> {
        self.0.insert(Key::from(index)).map_err(Into::into)
    }

    /// Calls insert on each cell index in the iterator.
    ///
    /// If an error occurred while adding an element, processing is stopped
    /// and the error is returned.
    pub fn extend_iter(
        &mut self,
        iter: impl IntoIterator<Item = CellIndex>,
    ) -> Result<(), BuildError> {
        self.0
            .extend_iter(iter.into_iter().map(Key::from))
            .map_err(Into::into)
    }

    /// Finishes the construction of the set and flushes the underlying
    /// writer. After completion, the data written to `W` may be read using
    /// one of `FrozenSet`'s constructor methods.
    pub fn finish(self) -> Result<(), BuildError> {
        self.0.finish().map_err(Into::into)
    }

    /// Just like `finish`, except it returns the underlying writer after
    /// flushing it.
    pub fn into_inner(self) -> Result<W, BuildError> {
        self.0.into_inner().map_err(Into::into)
    }
}

impl FrozenSetBuilder<Vec<u8>> {
    /// Create a builder that builds a set in memory.
    #[inline]
    pub fn memory() -> Self {
        Self(SetBuilder::memory())
    }

    /// Finishes the construction of the set and returns it.
    #[inline]
    pub fn into_set(self) -> FrozenSet<Vec<u8>> {
        FrozenSet(self.0.into_set())
    }
}

// ------------------------------------------------------------------------------

/// An iterator which counts from one to five
pub struct FrozenSetIterator<'a> {
    stream: Stream<'a>,
    len: usize,
    count: usize,
}

impl<'a> FrozenSetIterator<'a> {
    pub fn new<D>(set: &'a FrozenSet<D>) -> Self
    where
        D: AsRef<[u8]>,
    {
        Self {
            stream: set.0.stream(),
            len: set.len(),
            count: 0,
        }
    }
}

impl Iterator for FrozenSetIterator<'_> {
    type Item = CellIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.stream.next().map(|key| {
            self.count += 1;
            Key::from(key).into()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len(), Some(self.len()))
    }
}

impl ExactSizeIterator for FrozenSetIterator<'_> {
    // We can easily calculate the remaining number of iterations.
    fn len(&self) -> usize {
        self.len - self.count
    }
}
