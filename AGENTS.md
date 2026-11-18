# Working on this native Horizon mod (SDK layout, 2026-11-18)

This port is **one flat SDK package**: `horizon-mod.toml` + `Cargo.toml` + `src/lib.rs`
(the one `impl Hooks`) + `src/runtime.rs` (the pure logic and its tests) + `assets/` +
`data/`. The launcher compiles it against the build `[native]` pins; `.horizon/sdk/` is
fetched per clone and never packed.

Migrated from the retired `.hnmod` container (Horizon's mod-API merge, #16941). Do **not**
recreate `mixins/<id>/`, `horizon-mixin.toml`, `horizon-native.json`, `compatibility.json`,
`rust-toolchain.toml`, `tools/horizon.py`, `tools/package.py` or a plan/injection schema:
`ports/_shared/AGENT_MOD_STRUCTURE.md` §0 supersedes them. The retired tree is kept as
evidence in `../_shared/evidence/artifacts/retired-hnmod-0.1.0/`.

- **`src/runtime.rs` is the pure core** and must stay free of `mod_api` types: it holds
  the ids, the config, the attribute table and the lightning decision, and `cargo test`
  exercises it without a game. Carry it byte-identical between format changes.
- **`src/lib.rs` is glue only**: the item keys, the config file, the two host writes
  (attribute modifiers, an equipment move) and the damage hook. No numbers belong here.
- Comments explain *why*; behaviour belongs in `runtime.rs` and its tests.

## Validate

```sh
CARGO_INCREMENTAL=0 cargo test
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check                 # never --all: it also formats the fetched SDK
rm -f Cargo.lock                  # a packed lock changes the artifact's sha256
CARGO_INCREMENTAL=0 cargo horizon-mod build
cargo horizon-mod check artifacts.hmod
cargo horizon-mod fits artifacts.hmod --build nightly
```

`build-slot.sh` speaks the retired staged tool (`horizon.py stage/check/test/build`) and
does not apply to this layout; the commands above run directly in the port.

Do not modify the pinned Horizon or horizon-mixins checkouts, do not commit `.horizon/`,
`target/` or `Cargo.lock`, and keep the delivered `artifacts.hmod` in this directory (never
in `target/`). Keep validation claims and the supported revisions honest.
