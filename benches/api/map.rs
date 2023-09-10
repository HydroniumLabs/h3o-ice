use super::utils::load_dataset;
use criterion::{black_box, BenchmarkId, Criterion};
use h3o::{CellIndex, Resolution};
use h3o_ice::FrozenMap;

pub fn build(c: &mut Criterion) {
    let cells = load_dataset("Paris");
    let expanded = CellIndex::uncompact(cells.iter().copied(), Resolution::Ten)
        .enumerate()
        .map(|(idx, cell)| (cell, idx as u64))
        .collect::<Vec<_>>();
    let compacted = cells
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, cell)| (cell, idx as u64))
        .collect::<Vec<_>>();

    let mut group = c.benchmark_group("Map/Build");
    group.bench_function("Expanded", |b| {
        b.iter(|| FrozenMap::try_from_iter(black_box(expanded.iter().copied())))
    });
    group.bench_function("Compacted", |b| {
        b.iter(|| {
            FrozenMap::try_from_iter(black_box(compacted.iter().copied()))
        })
    });
    group.finish();
}

pub fn contains_key(c: &mut Criterion) {
    let cells = load_dataset("France");
    let compacted = FrozenMap::try_from_iter(
        cells
            .iter()
            .copied()
            .enumerate()
            .map(|(idx, cell)| (cell, idx as u64)),
    )
    .expect("compacted map");
    let expanded = FrozenMap::try_from_iter(
        CellIndex::uncompact(cells, Resolution::Ten)
            .enumerate()
            .map(|(idx, cell)| (cell, idx as u64)),
    )
    .expect("expanded map");

    let cells = [
        0x8aa304772a2ffff,
        0x8aa30476d1affff,
        0x8aa304676c4ffff,
        0x8aa30460c177fff,
        0x8aa250a2010ffff,
        0x8a3961c0a72ffff,
        0x8a186918d8d7fff,
        0x8a1fb6b1a20ffff,
    ]
    .iter()
    .map(|&hex| CellIndex::try_from(hex).expect("valid cell index"))
    .collect::<Vec<_>>();

    let mut group = c.benchmark_group("Map/ContainsKey");
    for (i, cell) in cells.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("Expanded", i),
            cell,
            |b, cell| b.iter(|| expanded.contains_key(*cell)),
        );
        group.bench_with_input(
            BenchmarkId::new("Compacted", i),
            cell,
            |b, cell| b.iter(|| compacted.contains_key(*cell)),
        );
    }
    group.finish();
}

pub fn get(c: &mut Criterion) {
    let cells = load_dataset("France");
    let compacted = FrozenMap::try_from_iter(
        cells
            .iter()
            .copied()
            .enumerate()
            .map(|(idx, cell)| (cell, idx as u64)),
    )
    .expect("compacted map");
    let expanded = FrozenMap::try_from_iter(
        CellIndex::uncompact(cells, Resolution::Ten)
            .enumerate()
            .map(|(idx, cell)| (cell, idx as u64)),
    )
    .expect("expanded map");

    let cells = [
        0x8aa304772a2ffff,
        0x8aa30476d1affff,
        0x8aa304676c4ffff,
        0x8aa30460c177fff,
        0x8aa250a2010ffff,
        0x8a3961c0a72ffff,
        0x8a186918d8d7fff,
        0x8a1fb6b1a20ffff,
    ]
    .iter()
    .map(|&hex| CellIndex::try_from(hex).expect("valid cell index"))
    .collect::<Vec<_>>();

    let mut group = c.benchmark_group("Map/Get");
    for (i, cell) in cells.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("Expanded", i),
            cell,
            |b, cell| b.iter(|| expanded.get(*cell)),
        );
        group.bench_with_input(
            BenchmarkId::new("Compacted", i),
            cell,
            |b, cell| b.iter(|| compacted.get(*cell)),
        );
    }
    group.finish();
}

pub fn range(c: &mut Criterion) {
    let cells = load_dataset("France");
    let expanded = FrozenMap::try_from_iter(
        CellIndex::uncompact(cells, Resolution::Ten)
            .enumerate()
            .map(|(idx, cell)| (cell, idx as u64)),
    )
    .expect("expanded map");

    let cells = [
        0x8aa304772a2ffff,
        0x89a30476d1bffff,
        0x88a304676dfffff,
        0x87a30460cffffff,
        0x86a250a27ffffff,
        0x855f159bfffffff,
        0x845f155ffffffff,
        0x833964fffffffff,
    ]
    .iter()
    .map(|&hex| CellIndex::try_from(hex).expect("valid cell index"))
    .collect::<Vec<_>>();

    let mut group = c.benchmark_group("Map/Range");
    for (i, cell) in cells.iter().enumerate() {
        group.bench_with_input(i.to_string(), cell, |b, cell| {
            b.iter(|| expanded.descendants(*cell).for_each(drop))
        });
    }
    group.finish();
}
