# h3o-ice — Frozen (aka read-only) sets and maps for H3 cell index

[![Crates.io](https://img.shields.io/crates/v/h3o-ice.svg)](https://crates.io/crates/h3o-ice)
[![Docs.rs](https://docs.rs/h3o-ice/badge.svg)](https://docs.rs/h3o-ice)
[![CI Status](https://github.com/HydroniumLabs/h3o-ice/actions/workflows/ci.yml/badge.svg)](https://github.com/HydroniumLabs/h3o-ice/actions)
[![Coverage](https://img.shields.io/codecov/c/github/HydroniumLabs/h3o-ice)](https://app.codecov.io/gh/HydroniumLabs/h3o-ice)
[![License](https://img.shields.io/badge/license-BSD-green)](https://opensource.org/licenses/BSD-3-Clause)

Those data structures are built on top of finite state transducers, which allows
them to be extremely compact while offering fast lookup.

This is especially for read-only indexes (both in-memory or on-disk).

## Installation

### Cargo

* Install the rust toolchain in order to have cargo installed by following
  [this](https://www.rust-lang.org/tools/install) guide.
* run `cargo install h3o-ice`

## Usage

```rust
use h3o::{LatLng, Resolution};
use h3o_ice::FrozenSet;

let coord = LatLng::new(48.872280706 2.332697839).expect("valid coord");
let set = FrozenSet::try_from_iter(
    coord.to_cell(Resolution::Nine)
        .children(Resolution::Ten)
)
.expect("failed to create set");

set.contains(CellIndex::try_from(0x8a1fb4666417fff).expect("valid cell"));
```

## Comparison with other data structures

|                      | `Frozen{Set,Map}`      | `BTree{Set,Map}` | `Hash{Set,Map}`  |
| -------------------- | ---------------------- | ---------------- | ---------------- |
| Memory overhead      | ✅ (negative [^1])     | ❌ (50%[^2])     | ❌ (12-125%[^2]) |
| Search complexity    | _O(key size)_          | _O(log #items)_  | _O(1)_           |
| Range query          | ✅                     | ✅               | ❌               |
| Prefix lookup        | ✅                     | ❌               | ❌               |
| Insertion/Deletion   | ❌                     | ✅               | ✅               |
| Direct lookup        | 163 ns                 | 55 ns            | 19 ns            |
| Compacted lookup     | 67 ns                  | 401 ns           | 125 ns           |


About the lookup time:
- input dataset is France coverage at resolution 10:
    - raw dataset: 44 250 550 cells (333.60M).
    - compacted (i.e. replacing clusters of cells their ancestors): 127 264 cells (0.97M)
- Lookup of a resolution 10 cell:
    - single lookup in the raw dataset.
    - prefix lookup (or repetitive lookup if not supported) in the compacted dataset.

You can run the provided benchmarks to get measures relevant to your machine.

To sum up:
- if you can afford the memory usage and don't care about ordering (e.g. range
  query) go with a `HashSet` for maximum speed.
- if you need ordering but don't need to work on compacted set, go with a
  `BTreeSet`.
- if you have a large read-only dataset, wants to optimize memory usage and be
  able to query compacted data directly then `FrozenSet` is your friend.

`Frozen{Map,Set}` are not general purpose data structures. They come with a
bunch of extra constraints (read-only, must be build from pre-sorted input, ...)
but they really shine in their niche and provides a bunch a goodies (e.g. key
compressions, can be mmap'd, prefix lookup, ...).

## License

[BSD 3-Clause](./LICENSE)

[^1]: `Frozen{Set,Map}` compresses both common prefixes and suffixes of keys, resulting in a smaller size than the input data set (e.g. the 333MB test dataset fits into a 176KB `FrozenSet`)
[^2]: [Measuring the overhead of HashMaps in Rust](https://ntietz.com/blog/rust-hashmap-overhead/)
