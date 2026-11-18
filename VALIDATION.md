# Validation

## 0.2.0 — the SDK package (2026-11-18)

Validated on Linux with rustc 1.97.0, edition 2024, against the pinned SDK of Horizon
`0b14f8acfd97fa2c750e42795249dcc981178ca4` + horizon-mixins
`e59da9094f0c2fdda6c627fe9eff22dec6a906f0` (`horizon-mod.toml` `[native]`). Every
command ran **directly in `ports/artifacts`** with `CARGO_INCREMENTAL=0`: the SDK layout
has no staged tree, and `build-slot.sh` speaks the retired `horizon.py` tool.

| Check | Command | Result |
|---|---|---|
| Formatting | `cargo fmt --check` (not `--all`: it would format the fetched SDK) | exit 0 |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** |
| Unit tests | `cargo test` | **21 passed**, 0 failed |
| Package | `rm -f Cargo.lock; cargo horizon-mod build` | `Built ./target/horizon-mod/artifacts.hmod`, exit 0 |
| Registry check | `cargo horizon-mod check artifacts.hmod` | `artifacts 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)`, 23,267 B, sha256 `72ebd4a2…cae3`, exit 0 |
| Pinned build | `cargo horizon-mod fits artifacts.hmod --build nightly` | `0b14f8a  made for`, exit 0 |
| `runtime.rs` | `unzip -p artifacts.hmod src/runtime.rs \| sha256sum` | `7f23ded0…2207`, byte-identical to the retired `mixins/artifacts/runtime.rs` |
| Reproducible | two consecutive builds | same 23,267 B, same sha256 |

Evidence: `ports/_shared/evidence/artifacts/sdk-*.log`, `sdk-uses.json`,
`sdk-sha256.txt`, `sdk-entries.txt`, `sdk-runtime-cmp.txt`.

**Not re-run:** the in-game e2e. It needs a *launcher-built* game carrying the package
(`horizon-launcher native-build …`), which is not installed here; `tools/e2e.sh` is kept
and updated for the one-package layout (its `GAME` must be that build, and its config
file now lives in the game folder's `config/`). The 0.1.0 runs below stand as the
behaviour's evidence — the rules are the same file, byte for byte.

## 0.1.0 — the retired `.hnmod` bundle

Validated on Linux with Rust 1.95.0 on **2026-11-09**, against Horizon
`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9` + horizon-mixins
`5a64af938ec86b00d53d16a408aabc0038fb3e57` (the exact revisions in the retired
`compatibility.json`). The previous record was the `83ad384f` pin.

| Check | Result |
|---|---|
| `cargo fmt` + `cargo fmt --check` | exit 0 |
| `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| `cargo test --locked --offline` | 21 passed, 0 failed |
| `python3 tools/horizon.py stage` | pinned Horizon export, 89 s |
| `python3 tools/horizon.py check` | exit 0, 0 warnings, 53 s |
| `python3 tools/horizon.py test` | embedded `game::artifacts::tests`: 21 passed, 1240 filtered |
| `python3 tools/horizon.py build` | fast client binary, exit 0 (479 s) |
| `tools/package.sh` | `.hmod` 4,492 B `d2136172…b7321d` (15 entries); `.hnmod` with the content embedded 14,874 B `ca84a1cb…8b8e9` (10 entries, incl. `mixins/artifacts/data/artifacts-content.hmod`); a second run reproduced both |
| `cargo horizon-mod check artifacts.hnmod --kind native-mod` | exit 0, `native-mod … for 26.3` |
| `tools/e2e.sh` | `PASS` + `EXIT:0` (`ports/_shared/evidence/artifacts/e2e.log`) |
| `SELFTEST=1 tools/e2e.sh` | `FAIL` + `EXIT:1` (`e2e-negative.log`) |
| `EMBEDDED=1 tools/e2e.sh` (no `--dev-mod`) | `PASS` + `EXIT:0` (`e2e-embedded.log`), control leg shows the ids unknown |
| `EMBEDDED=1 SELFTEST=1 tools/e2e.sh` | `FAIL` + `EXIT:1` (`e2e-embedded-selftest.log`) |
| `ports/tools/check-e2e-logs.py artifacts --strict` | 4/4 ok |

The retired tooling (`tools/horizon.py`, `tools/package.sh`, `tools/test_stage.py`,
`tools/test_template.py`, `tools/audit/`) is kept under
`ports/_shared/evidence/artifacts/retired-hnmod-0.1.0/tools/`; the 0.1.0 `.hmod` and
`.hnmod` are beside it. Those commands no longer run — the format they target is gone.

No Minecraft resources or player data in the repository are read or changed; e2e runs
use a scratch `HORIZON_HOME`/`XDG_*` under `/tmp/opencode`.
