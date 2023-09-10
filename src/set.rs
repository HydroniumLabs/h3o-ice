use crate::{BuildError, Key};
use either::Either;
use fst::{set::Stream, IntoStreamer, Set, SetBuilder, Streamer};
use h3o::CellIndex;
use std::{
    io,
    ops::{Bound, RangeBounds},
};

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
    /// # Errors
    ///
    /// The set must have been written with a compatible builder. If the format
    /// is invalid or if there is a mismatch between the API version of this
    /// library and the set, then an error is returned.
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

    /// Tests the membership of a single H3 cell index.
    ///
    /// Returns true if the cell index or one of its ancestor is present.
    pub fn contains(&self, index: CellIndex) -> Option<CellIndex> {
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

    /// Return a lexicographically ordered stream of every descendant (present
    /// in the set) of the given cell index.
    #[allow(clippy::missing_panics_doc)] // Expect don't need to be documented.
    pub fn descendants(
        &self,
        index: CellIndex,
    ) -> impl Iterator<Item = CellIndex> + '_ {
        index.resolution().succ().map_or_else(
            // If there is no lower resolution there can't be any descendants.
            || Either::Left(std::iter::empty()),
            |resolution| {
                let mut children = index.children(resolution);
                let start = children.next().expect("first child");
                let end = children.last().expect("last child");
                Either::Right(
                    self.range((Bound::Included(start), Bound::Included(end))),
                )
            },
        )
    }

    /// Return a lexicographically ordered stream of all cells in this set.
    #[must_use]
    pub fn iter(&self) -> FrozenSetIterator<'_> {
        FrozenSetIterator::new(self)
    }

    /// Return a lexicographically ordered stream over the subset of keys the
    /// specified range.
    pub fn range(
        &self,
        range: impl RangeBounds<CellIndex>,
    ) -> impl Iterator<Item = CellIndex> + '_ {
        let (start, end) = (range.start_bound(), range.end_bound());

        if matches!((start, end), (Bound::Unbounded, Bound::Unbounded)) {
            return Either::Left(self.iter());
        }
        let builder = self.0.range();
        let builder = match start {
            Bound::Included(lower) => builder.ge(Key::from(*lower)),
            Bound::Excluded(lower) => builder.gt(Key::from(*lower)),
            Bound::Unbounded => builder,
        };
        let builder = match end {
            Bound::Included(upper) => builder.le(Key::from(*upper)),
            Bound::Excluded(upper) => builder.lt(Key::from(*upper)),
            Bound::Unbounded => builder,
        };
        Either::Right(FrozenSetRangeIterator::new(builder.into_stream()))
    }
}

impl FrozenSet<Vec<u8>> {
    /// Create a `FrozenSet` from an iterator of ordered H3 cell indexes.
    ///
    /// Note that this is a convenience function to build a set in memory.
    /// To build a set that streams to an arbitrary `io::Write`, use
    /// `FrozenSetBuilder`.
    ///
    /// # Errors
    ///
    /// If the iterator does not yield values in lexicographic order, then an
    /// error is returned.
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = CellIndex>,
    ) -> Result<Self, BuildError> {
        let mut builder = FrozenSetBuilder::memory();
        builder.extend_iter(iter)?;
        Self::new(builder.into_inner()?)
    }

    /// Returns the binary contents of this set.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_fst().as_bytes()
    }
}

// ------------------------------------------------------------------------------

/// A builder for creating a frozen set.
pub struct FrozenSetBuilder<W>(SetBuilder<W>);

impl<W: io::Write> FrozenSetBuilder<W> {
    /// Create a builder that builds a set by writing it to `wtr` in a
    /// streaming fashion.
    ///
    /// # Errors
    ///
    /// If there was a problem writing to the underlying writer, an error is
    /// returned.
    pub fn new(wtr: W) -> Result<Self, BuildError> {
        SetBuilder::new(wtr).map(Self).map_err(Into::into)
    }

    /// Insert a new cell index into the set.
    ///
    /// # Errors
    ///
    /// If a cell index is inserted that is less than any previous cell index
    /// added, then an error is returned.
    ///
    /// Similarly, if there was a problem writing to the underlying writer, an
    /// error is returned.
    pub fn insert(&mut self, index: CellIndex) -> Result<(), BuildError> {
        self.0.insert(Key::from(index)).map_err(Into::into)
    }

    /// Calls insert on each cell index in the iterator.
    ///
    /// # Errors
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

impl FrozenSetBuilder<Vec<u8>> {
    /// Create a builder that builds a set in memory.
    #[inline]
    #[must_use]
    pub fn memory() -> Self {
        Self(SetBuilder::memory())
    }

    /// Finishes the construction of the set and returns it.
    #[inline]
    #[must_use]
    pub fn into_set(self) -> FrozenSet<Vec<u8>> {
        FrozenSet(self.0.into_set())
    }
}

// ------------------------------------------------------------------------------

/// An iterator over the every value of the set.
pub struct FrozenSetIterator<'a> {
    stream: Stream<'a>,
    len: usize,
    count: usize,
}

impl<'a> FrozenSetIterator<'a> {
    fn new<D>(set: &'a FrozenSet<D>) -> Self
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

// ------------------------------------------------------------------------------

/// An iterator over a subset of keys in a specified range.
struct FrozenSetRangeIterator<'a> {
    stream: Stream<'a>,
}

impl<'a> FrozenSetRangeIterator<'a> {
    const fn new(stream: Stream<'a>) -> Self {
        Self { stream }
    }
}

impl Iterator for FrozenSetRangeIterator<'_> {
    type Item = CellIndex;

    fn next(&mut self) -> Option<Self::Item> {
        self.stream.next().map(|key| Key::from(key).into())
    }
}
