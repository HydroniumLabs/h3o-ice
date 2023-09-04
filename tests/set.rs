use h3o::{CellIndex, Resolution};
use h3o_ice::{FrozenSet, FrozenSetBuilder};
use std::{error::Error, io::Cursor};

#[test]
fn len() {
    let empty = FrozenSet::try_from_iter(std::iter::empty())
        .expect("failed to create set");
    assert_eq!(empty.len(), 0, "empty set");

    let single = FrozenSet::try_from_iter(std::iter::once(
        CellIndex::try_from(0x85318d83fffffff).expect("invalid cell"),
    ))
    .expect("failed to create set");
    assert_eq!(single.len(), 1, "single element");

    let multiple = FrozenSet::try_from_iter(
        CellIndex::try_from(0x85318d83fffffff)
            .expect("invalid cell")
            .children(Resolution::Seven),
    )
    .expect("failed to create set");
    assert_eq!(multiple.len(), 49, "multiple elements");
}

#[test]
fn is_empty() {
    let empty = FrozenSet::try_from_iter(std::iter::empty())
        .expect("failed to create set");
    assert!(empty.is_empty(), "empty set");

    let single = FrozenSet::try_from_iter(std::iter::once(
        CellIndex::try_from(0x85318d83fffffff).expect("invalid cell"),
    ))
    .expect("failed to create set");
    assert!(!single.is_empty(), "single element");
}

#[test]
fn contains() {
    let cell = CellIndex::try_from(0x85318d83fffffff).expect("invalid cell");
    let child = CellIndex::try_from(0x86318d837ffffff).expect("invalid cell");
    let descendant =
        CellIndex::try_from(0x89318d8368bffff).expect("invalid cell");
    let set = FrozenSet::try_from_iter(std::iter::once(cell))
        .expect("failed to create set");

    // Exact containment works.
    assert_eq!(set.contains(cell), Some(cell), "exact match");
    // Child containment works too.
    assert_eq!(set.contains(child), Some(cell), "direct child");
    // Even through multiple levels.
    assert_eq!(set.contains(descendant), Some(cell), "descendant");

    let not_related =
        CellIndex::try_from(0x85283473fffffff).expect("invalid cell");
    assert!(set.contains(not_related).is_none(), "not related");
}

#[test]
fn load_from_bytes() {
    let mut builder = FrozenSetBuilder::memory();
    builder
        .insert(CellIndex::try_from(0x85283473fffffff).expect("invalid cell"))
        .expect("failed to insert");
    builder
        .extend_iter(
            CellIndex::try_from(0x85318d83fffffff)
                .expect("invalid cell")
                .children(Resolution::Seven),
        )
        .expect("failed to extend");
    let expected = builder.into_set();
    let bytes = expected.as_bytes();

    let result = FrozenSet::new(bytes).expect("valid set");

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
        .insert(CellIndex::try_from(0x85283473fffffff).expect("invalid cell"))
        .expect("failed to insert");
    builder
        .extend_iter(
            CellIndex::try_from(0x85318d83fffffff)
                .expect("invalid cell")
                .children(Resolution::Seven),
        )
        .expect("failed to extend");
    builder.finish().expect("flushing set");
}

#[test]
fn wrong_order() {
    let cell1 = CellIndex::try_from(0x85318d83fffffff).expect("invalid cell");
    let cell2 = CellIndex::try_from(0x85283473fffffff).expect("invalid cell");

    let mut builder = FrozenSetBuilder::memory();
    builder.insert(cell1).expect("failed to insert");
    let err = builder.insert(cell2).expect_err("inserted out of order");

    assert!(err.source().is_some(), "preserve root cause");
    assert!(!err.to_string().is_empty(), "non-empty error");
}
