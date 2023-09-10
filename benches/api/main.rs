use criterion::{criterion_group, criterion_main};

mod map;
mod set;
mod utils;

criterion_group!(
    benches,
    set::build,
    set::contains,
    set::range,
    map::build,
    map::contains_key,
    map::get,
    map::range
);
criterion_main!(benches);
