use criterion::{
    black_box, criterion_group, criterion_main, BenchmarkId, Criterion,
};
use h3o::{CellIndex, Resolution};
use h3o_ice::FrozenSet;
use std::path::PathBuf;

fn build(c: &mut Criterion) {
    let compacted = load_dataset("Paris");
    let expanded =
        CellIndex::uncompact(compacted.iter().copied(), Resolution::Ten)
            .collect::<Vec<_>>();

    let mut group = c.benchmark_group("Build");
    group.bench_function("Expanded", |b| {
        b.iter(|| FrozenSet::try_from_iter(black_box(expanded.iter().copied())))
    });
    group.bench_function("Compacted", |b| {
        b.iter(|| {
            FrozenSet::try_from_iter(black_box(compacted.iter().copied()))
        })
    });
    group.finish();
}

fn lookup(c: &mut Criterion) {
    let dataset = load_dataset("France");
    let compacted = FrozenSet::try_from_iter(dataset.iter().copied())
        .expect("compacted set");
    let expanded = FrozenSet::try_from_iter(CellIndex::uncompact(
        dataset,
        Resolution::Ten,
    ))
    .expect("expanded set");

    let cells = [
        0x8aa304772a2ffff,
        0x8aa30476d1affff,
        0x8aa304676c4ffff,
        0x8aa30460c177fff,
        0x8aa250a2010ffff,
        0x8a3961c0a72ffff,
        0x8a186918d8d7fff,
        0x8b1fb6b1a20dfff,
    ]
    .iter()
    .map(|&hex| CellIndex::try_from(hex).expect("valid cell index"))
    .collect::<Vec<_>>();

    let mut group = c.benchmark_group("Lookup");
    for (i, cell) in cells.iter().enumerate() {
        group.bench_with_input(
            BenchmarkId::new("Expanded", i),
            cell,
            |b, cell| b.iter(|| expanded.contains(*cell)),
        );
        group.bench_with_input(
            BenchmarkId::new("Compacted", i),
            cell,
            |b, cell| b.iter(|| compacted.contains(*cell)),
        );
    }
    group.finish();
}

fn range(c: &mut Criterion) {
    let dataset = load_dataset("France");
    let expanded = FrozenSet::try_from_iter(CellIndex::uncompact(
        dataset,
        Resolution::Ten,
    ))
    .expect("expanded set");

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

    let mut group = c.benchmark_group("Range");
    for (i, cell) in cells.iter().enumerate() {
        group.bench_with_input(i.to_string(), cell, |b, cell| {
            b.iter(|| expanded.descendants(*cell).for_each(drop))
        });
    }
    group.finish();
}

criterion_group!(benches, build, lookup, range);
criterion_main!(benches);

pub fn load_dataset(name: &str) -> Vec<h3o::CellIndex> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let filepath = format!("dataset/{name}.thc");
    path.push(filepath);

    let bytes = std::fs::read(path).expect("read test data");
    thc::decompress(&bytes)
        .collect::<Result<_, _>>()
        .expect("unpack test data")
}
