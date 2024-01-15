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
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use h3o_ice::FrozenSet;
    /// use std::fs;
    ///
    /// # let file_path = "";
    /// let bytes = fs::read_to_string(file_path)?;
    /// let set = FrozenSet::new(bytes);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn new(data: D) -> Result<Self, BuildError> {
        Ok(Set::new(data).map(Self)?)
    }

    /// Returns the number of elements in this set.
    ///
    /// # Examples
    ///
    /// ```
    /// use h3o::CellIndex;
    /// use h3o_ice::FrozenSet;
    ///
    /// let index = CellIndex::try_from(0x8a1fb46622dffff)?;
    /// let set = FrozenSet::try_from_iter(std::iter::once(index))?;
    /// assert_eq!(set.len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns true if and only if this set is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use h3o::CellIndex;
    /// use h3o_ice::FrozenSet;
    ///
    /// let set = FrozenSet::try_from_iter(std::iter::empty())?;
    /// assert!(set.is_empty());
    ///
    /// let index = CellIndex::try_from(0x8a1fb46622dffff)?;
    /// let set = FrozenSet::try_from_iter(std::iter::once(index))?;
    /// assert!(!set.is_empty());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Tests the membership of a single H3 cell index.
    ///
    /// Returns true if the cell index or one of its ancestor is present.
    ///
    /// # Examples
    ///
    /// ```
    /// use h3o::CellIndex;
    /// use h3o_ice::FrozenSet;
    ///
    /// let cell = CellIndex::try_from(0x8a1fb46622dffff)?;
    /// let set = FrozenSet::try_from_iter(std::iter::once(cell))?;
    ///
    /// // Exact membership works.
    /// assert_eq!(set.contains(cell), Some(cell));
    ///
    /// // Child membership works too.
    /// let child = CellIndex::try_from(0x8b1fb46622d8fff)?;
    /// assert_eq!(set.contains(child), Some(cell));
    ///
    /// // Even through multiple levels.
    /// let descendant = CellIndex::try_from(0x8d1fb46622d85bf)?;
    /// assert_eq!(set.contains(descendant), Some(cell));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use h3o::{CellIndex, Resolution};
    /// use h3o_ice::FrozenSet;
    ///
    /// let index = CellIndex::try_from(0x85318d83fffffff)?;
    /// let set = FrozenSet::try_from_iter(index.children(Resolution::Six))?;
    ///
    /// for cell in set.descendants(index) {
    ///     println!("{cell}");
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use h3o::{CellIndex, Resolution};
    /// use h3o_ice::FrozenSet;
    ///
    /// let index = CellIndex::try_from(0x85318d83fffffff)?;
    /// let set = FrozenSet::try_from_iter(index.children(Resolution::Six))?;
    ///
    /// for cell in set.iter() {
    ///     println!("{cell}");
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn iter(&self) -> FrozenSetIterator<'_> {
        FrozenSetIterator::new(self)
    }

    /// Return a lexicographically ordered stream over the subset of keys the
    /// specified range.
    ///
    /// # Examples
    ///
    /// ```
    /// use h3o::{CellIndex, Resolution};
    /// use h3o_ice::FrozenSet;
    /// use std::ops::Bound;
    ///
    /// let index = CellIndex::try_from(0x85318d83fffffff)?;
    /// let set = FrozenSet::try_from_iter(index.children(Resolution::Six))?;
    ///
    /// let start = Bound::Included(CellIndex::try_from(0x86318d817ffffff)?);
    /// let end = Bound::Excluded(CellIndex::try_from(0x86318d827ffffff)?);
    ///
    /// for cell in set.range((start, end)) {
    ///     println!("{cell}");
    /// }
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use h3o::{CellIndex, Resolution};
    /// use h3o_ice::FrozenSet;
    ///
    /// let index = CellIndex::try_from(0x85318d83fffffff)?;
    /// let set = FrozenSet::try_from_iter(index.children(Resolution::Six))?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn try_from_iter(
        iter: impl IntoIterator<Item = CellIndex>,
    ) -> Result<Self, BuildError> {
        let mut builder = FrozenSetBuilder::memory();
        builder.extend_iter(iter)?;
        Self::new(builder.into_inner()?)
    }

    /// Returns the binary contents of this set.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use h3o::{CellIndex, Resolution};
    /// use h3o_ice::FrozenSet;
    ///
    /// let index = CellIndex::try_from(0x85318d83fffffff)?;
    /// let set = FrozenSet::try_from_iter(index.children(Resolution::Six))?;
    ///
    /// # let file_path = "";
    /// std::fs::write(file_path, set.as_bytes())?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_fst().as_bytes()
    }
}

impl<'a, D: AsRef<[u8]>> IntoIterator for &'a FrozenSet<D> {
    type IntoIter = FrozenSetIterator<'a>;
    type Item = CellIndex;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

// ------------------------------------------------------------------------------

/// A builder for creating a frozen set.
///
/// # Example: build in memory
///
/// ```
/// use h3o::{CellIndex, Resolution};
/// use h3o_ice::FrozenSetBuilder;
///
/// let mut builder = FrozenSetBuilder::memory();
/// builder.insert(CellIndex::try_from(0x85283473fffffff)?)?;
///
/// let index = CellIndex::try_from(0x8a1fb46622dffff)?;
/// builder.extend_iter(index.children(Resolution::Six))?;
///
/// let set = builder.into_set();
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
/// # Example: stream to file
///
/// ```no_run
/// use h3o::{CellIndex, Resolution};
/// use h3o_ice::FrozenSetBuilder;
/// use std::{fs, io};
///
/// # let file_path = "";
/// let mut wtr = io::BufWriter::new(fs::File::create(file_path)?);
/// let mut builder = FrozenSetBuilder::new(wtr)?;
///
/// let index = CellIndex::try_from(0x8a1fb46622dffff)?;
/// builder.extend_iter(index.children(Resolution::Six))?;
///
/// builder.finish()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
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
