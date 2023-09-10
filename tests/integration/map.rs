use crate::cell_index;
use h3o::{CellIndex, Resolution};
use h3o_ice::{FrozenMap, FrozenMapBuilder};
use std::{error::Error, io::Cursor, ops::Bound};

#[test]
fn len() {
    let empty = FrozenMap::try_from_iter(std::iter::empty())
        .expect("failed to create map");
    assert_eq!(empty.len(), 0, "empty map");

    let single = FrozenMap::try_from_iter(std::iter::once((
        cell_index!(0x8a1fb46622dffff),
        42,
    )))
    .expect("failed to create map");
    assert_eq!(single.len(), 1, "single element");

    let multiple =
        FrozenMap::try_from_iter(test_cells()).expect("failed to create map");
    assert_eq!(multiple.len(), 49, "multiple elements");
}

#[test]
fn is_empty() {
    let empty = FrozenMap::try_from_iter(std::iter::empty())
        .expect("failed to create map");
    assert!(empty.is_empty(), "empty map");

    let single = FrozenMap::try_from_iter(std::iter::once((
        cell_index!(0x8a1fb46622dffff),
        42,
    )))
    .expect("failed to create map");
    assert!(!single.is_empty(), "single element");
}

#[test]
fn contains_key() {
    let cell = cell_index!(0x8a1fb46622dffff);
    let child = cell_index!(0x8b1fb46622d8fff);
    let descendant = cell_index!(0x8d1fb46622d85bf);
    let map = FrozenMap::try_from_iter(std::iter::once((cell, 33)))
        .expect("failed to create map");

    // Exact membership works.
    assert_eq!(map.contains_key(cell), Some(cell), "exact match");
    // Child membership works too.
    assert_eq!(map.contains_key(child), Some(cell), "direct child");
    // Even through multiple levels.
    assert_eq!(map.contains_key(descendant), Some(cell), "descendant");

    let not_related = cell_index!(0x85283473fffffff);
    assert!(map.contains_key(not_related).is_none(), "not related");
}

#[test]
fn get() {
    let cell = cell_index!(0x8a1fb46622dffff);
    let ancestor = cell_index!(0x85283473fffffff);
    let child = cell_index!(0x8a2834701ab7fff);
    let map = FrozenMap::try_from_iter(vec![(cell, 33), (ancestor, 1024)])
        .expect("failed to create map");

    // Exact lookup works.
    assert_eq!(map.get(cell), Some((cell, 33)), "exact match");
    // Lookup with a descendant too.
    assert_eq!(map.get(child), Some((ancestor, 1024)), "descendant");

    let not_related = cell_index!(0x8aa88b946a27fff);
    assert!(map.get(not_related).is_none(), "not related");
}

#[test]
fn load_from_bytes() {
    // Build map in memory.
    let mut builder = FrozenMapBuilder::memory();
    builder
        .insert(cell_index!(0x85283473fffffff), 42)
        .expect("failed to insert");
    builder.extend_iter(test_cells()).expect("failed to extend");
    let expected = builder.into_map();

    // Get the underlying bytes.
    let bytes = expected.as_bytes();

    // Rebuild the map from thoses bytes.
    let result = FrozenMap::new(bytes).expect("valid map");

    // Data is exactly the same.
    assert_eq!(
        expected.iter().collect::<Vec<_>>(),
        result.iter().collect::<Vec<_>>()
    );
}

#[test]
fn io_build() {
    let buffer = Cursor::new(Vec::new());
    let mut builder = FrozenMapBuilder::new(buffer).expect("builder");
    builder
        .insert(cell_index!(0x85283473fffffff), 42)
        .expect("failed to insert");
    builder.extend_iter(test_cells()).expect("failed to extend");
    builder.finish().expect("flushing map");
}

#[test]
fn keys() {
    let map =
        FrozenMap::try_from_iter(test_cells()).expect("failed to create map");
    let result = map.keys().collect::<Vec<_>>();
    let expected = test_cells().map(|(cell, _)| cell).collect::<Vec<_>>();

    assert_eq!(result, expected);
}

#[test]
fn values() {
    let map =
        FrozenMap::try_from_iter(test_cells()).expect("failed to create map");
    let result = map.values().collect::<Vec<_>>();
    let expected = test_cells().map(|(_, value)| value).collect::<Vec<_>>();

    assert_eq!(result, expected);
}

#[test]
fn wrong_order() {
    // Building map from non-sorted input fails.
    let mut builder = FrozenMapBuilder::memory();
    builder
        .insert(cell_index!(0x85318d83fffffff), 42)
        .expect("failed to insert");
    let err = builder
        .insert(cell_index!(0x85283473fffffff), 33)
        .expect_err("inserted out of order");

    assert!(err.source().is_some(), "preserve root cause");
    assert!(!err.to_string().is_empty(), "non-empty error");
}

#[test]
fn range() {
    let map = FrozenMap::try_from_iter(
        cell_index!(0x85318d83fffffff)
            .children(Resolution::Six)
            .enumerate()
            .map(|(idx, cell)| (cell, idx as u64)),
    )
    .expect("failed to create map");

    // A (half-open) range bounded inclusively below and exclusively above.
    let result = map
        .range((
            Bound::Included(cell_index!(0x86318d817ffffff)),
            Bound::Excluded(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        (cell_index!(0x86318d817ffffff), 2),
        (cell_index!(0x86318d81fffffff), 3),
    ];
    assert_eq!(result, expected, "Range");

    // An open range bounded exclusively below and above.
    let result = map
        .range((
            Bound::Excluded(cell_index!(0x86318d817ffffff)),
            Bound::Excluded(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![(cell_index!(0x86318d81fffffff), 3)];
    assert_eq!(result, expected, "RangeOpen");

    // A range only bounded inclusively below.
    let result = map
        .range((
            Bound::Included(cell_index!(0x86318d817ffffff)),
            Bound::Unbounded,
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        (cell_index!(0x86318d817ffffff), 2),
        (cell_index!(0x86318d81fffffff), 3),
        (cell_index!(0x86318d827ffffff), 4),
        (cell_index!(0x86318d82fffffff), 5),
        (cell_index!(0x86318d837ffffff), 6),
    ];
    assert_eq!(result, expected, "RangeFrom");

    // An unbounded range.
    let result = map
        .range((Bound::<CellIndex>::Unbounded, Bound::Unbounded))
        .collect::<Vec<_>>();
    let expected = vec![
        (cell_index!(0x86318d807ffffff), 0),
        (cell_index!(0x86318d80fffffff), 1),
        (cell_index!(0x86318d817ffffff), 2),
        (cell_index!(0x86318d81fffffff), 3),
        (cell_index!(0x86318d827ffffff), 4),
        (cell_index!(0x86318d82fffffff), 5),
        (cell_index!(0x86318d837ffffff), 6),
    ];
    assert_eq!(result, expected, "RangeFull");

    // A range bounded inclusively below and above.
    let result = map
        .range((
            Bound::Included(cell_index!(0x86318d817ffffff)),
            Bound::Included(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        (cell_index!(0x86318d817ffffff), 2),
        (cell_index!(0x86318d81fffffff), 3),
        (cell_index!(0x86318d827ffffff), 4),
    ];
    assert_eq!(result, expected, "RangeInclusive");

    // A range only bounded exclusively above.
    let result = map
        .range((
            Bound::Unbounded,
            Bound::Excluded(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        (cell_index!(0x86318d807ffffff), 0),
        (cell_index!(0x86318d80fffffff), 1),
        (cell_index!(0x86318d817ffffff), 2),
        (cell_index!(0x86318d81fffffff), 3),
    ];
    assert_eq!(result, expected, "RangeTo");

    // A range only bounded inclusively above.
    let result = map
        .range((
            Bound::Unbounded,
            Bound::Included(cell_index!(0x86318d827ffffff)),
        ))
        .collect::<Vec<_>>();
    let expected = vec![
        (cell_index!(0x86318d807ffffff), 0),
        (cell_index!(0x86318d80fffffff), 1),
        (cell_index!(0x86318d817ffffff), 2),
        (cell_index!(0x86318d81fffffff), 3),
        (cell_index!(0x86318d827ffffff), 4),
    ];
    assert_eq!(result, expected, "RangeToInclusive");
}

// -----------------------------------------------------------------------------

fn test_cells() -> impl Iterator<Item = (h3o::CellIndex, u64)> {
    cell_index!(0x85318d83fffffff)
        .children(Resolution::Seven)
        .enumerate()
        .map(|(idx, cell)| (cell, idx as u64))
}
