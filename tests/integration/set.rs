use crate::cell_index;
use h3o::{CellIndex, Resolution};
use h3o_ice::{FrozenSet, FrozenSetBuilder};
use std::{error::Error, io::Cursor, ops::Bound};

#[test]
fn len() {
    let empty = FrozenSet::try_from_iter(std::iter::empty())
        .expect("failed to create set");
    assert_eq!(empty.len(), 0, "empty set");

    let single = FrozenSet::try_from_iter(std::iter::once(cell_index!(
        0x8a1fb46622dffff
    )))
    .expect("failed to create set");
    assert_eq!(single.len(), 1, "single element");

    let multiple =
        FrozenSet::try_from_iter(test_cells()).expect("failed to create set");
    assert_eq!(multiple.len(), 49, "multiple elements");
}

#[test]
fn is_empty() {
    let empty = FrozenSet::try_from_iter(std::iter::empty())
        .expect("failed to create set");
    assert!(empty.is_empty(), "empty set");

    let single = FrozenSet::try_from_iter(std::iter::once(cell_index!(
        0x8a1fb46622dffff
    )))
    .expect("failed to create set");
    assert!(!single.is_empty(), "single element");
}

#[test]
fn contains() {
    let cell = cell_index!(0x8a1fb46622dffff);
    let child = cell_index!(0x8b1fb46622d8fff);
    let descendant = cell_index!(0x8d1fb46622d85bf);
    let set = FrozenSet::try_from_iter(std::iter::once(cell))
        .expect("failed to create set");

    // Exact membership works.
    assert_eq!(set.contains(cell), Some(cell), "exact match");
    // Child membership works too.
    assert_eq!(set.contains(child), Some(cell), "direct child");
    // Even through multiple levels.
    assert_eq!(set.contains(descendant), Some(cell), "descendant");

    let not_related = cell_index!(0x85283473fffffff);
    assert!(set.contains(not_related).is_none(), "not related");
}

#[test]
fn load_from_bytes() {
    // Build set in memory.
    let mut builder = FrozenSetBuilder::memory();
    builder
        .insert(cell_index!(0x85283473fffffff))
        .expect("failed to insert");
    builder.extend_iter(test_cells()).expect("failed to extend");
    let expected = builder.into_set();

    // Get the underlying bytes.
    let bytes = expected.as_bytes();

    // Rebuild the set from thoses bytes.
    let result = FrozenSet::new(bytes).expect("valid set");

    // Data is exactly the same.
    assert_eq!(
        expected.iter().collect::<Vec<_>>(),
        result.iter().collect::<Vec<_>>()
    );
}

#[test]
fn io_build() {
    let buffer = Cursor::new(Vec::new());
    let mut builder = FrozenSetBuilder::new(buffer).expect("builder");
    builder
        .insert(cell_index!(0x85283473fffffff))
        .expect("failed to insert");
    builder.extend_iter(test_cells()).expect("failed to extend");
    builder.finish().expect("flushing set");
}

#[test]
fn wrong_order() {
    // Building set from non-sorted input fails.
    let mut builder = FrozenSetBuilder::memory();
    builder
        .insert(cell_index!(0x85318d83fffffff))
        .expect("failed to insert");
    let err = builder
        .insert(cell_index!(0x85283473fffffff))
        .expect_err("inserted out of order");

    assert!(err.source().is_some(), "preserve root cause");
    assert!(!err.to_string().is_empty(), "non-empty error");
}

#[test]
fn range() {
    let set = FrozenSet::try_from_iter(
        cell_index!(0x85318d83fffffff).children(Resolution::Six),
    )
    .expect("failed to create set");

    // A (half-open) range bounded inclusively below and exclusively above.
    let result = set
        .range((
            Bound::Included(cell_index!(0x86318d817ffffff)),
            Bound::Excluded(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        cell_index!(0x86318d817ffffff),
        cell_index!(0x86318d81fffffff),
    ];
    assert_eq!(result, expected, "Range");

    // An open range bounded exclusively below and above.
    let result = set
        .range((
            Bound::Excluded(cell_index!(0x86318d817ffffff)),
            Bound::Excluded(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![cell_index!(0x86318d81fffffff)];
    assert_eq!(result, expected, "RangeOpen");

    // A range only bounded inclusively below.
    let result = set
        .range((
            Bound::Included(cell_index!(0x86318d817ffffff)),
            Bound::Unbounded,
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        cell_index!(0x86318d817ffffff),
        cell_index!(0x86318d81fffffff),
        cell_index!(0x86318d827ffffff),
        cell_index!(0x86318d82fffffff),
        cell_index!(0x86318d837ffffff),
    ];
    assert_eq!(result, expected, "RangeFrom");

    // An unbounded range.
    let result = set
        .range((Bound::<CellIndex>::Unbounded, Bound::Unbounded))
        .collect::<Vec<_>>();
    let expected = vec![
        cell_index!(0x86318d807ffffff),
        cell_index!(0x86318d80fffffff),
        cell_index!(0x86318d817ffffff),
        cell_index!(0x86318d81fffffff),
        cell_index!(0x86318d827ffffff),
        cell_index!(0x86318d82fffffff),
        cell_index!(0x86318d837ffffff),
    ];
    assert_eq!(result, expected, "RangeFull");

    // A range bounded inclusively below and above.
    let result = set
        .range((
            Bound::Included(cell_index!(0x86318d817ffffff)),
            Bound::Included(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        cell_index!(0x86318d817ffffff),
        cell_index!(0x86318d81fffffff),
        cell_index!(0x86318d827ffffff),
    ];
    assert_eq!(result, expected, "RangeInclusive");

    // A range only bounded exclusively above.
    let result = set
        .range((
            Bound::Unbounded,
            Bound::Excluded(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        cell_index!(0x86318d807ffffff),
        cell_index!(0x86318d80fffffff),
        cell_index!(0x86318d817ffffff),
        cell_index!(0x86318d81fffffff),
    ];
    assert_eq!(result, expected, "RangeTo");

    // A range only bounded inclusively above.
    let result = set
        .range((
            Bound::Unbounded,
            Bound::Included(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        cell_index!(0x86318d807ffffff),
        cell_index!(0x86318d80fffffff),
        cell_index!(0x86318d817ffffff),
        cell_index!(0x86318d81fffffff),
        cell_index!(0x86318d827ffffff),
    ];
    assert_eq!(result, expected, "RangeToInclusive");
}

// -----------------------------------------------------------------------------

fn test_cells() -> impl Iterator<Item = h3o::CellIndex> {
    cell_index!(0x85318d83fffffff).children(Resolution::Seven)
}
