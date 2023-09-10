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

## License

[BSD 3-Clause](./LICENSE)
