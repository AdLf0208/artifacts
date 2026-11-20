# Artifacts — Horizon port

A clean-room Rust rewrite of
[Artifacts](https://github.com/ochotonida/artifacts) (MIT) as **one flat SDK
package** for Horizon at pin `0b14f8a` (nightly 2026-11-18), targeted at
Minecraft 26.3. Upstream read at the 1.21.1 branch `7cf7dc4` (13.2.5) and checked
against the 26.1 (`4b67278`, 15.1.3) and 26.3 (`27bab87`) branches, which behave
identically for these three items.

| Item | Effect while worn in the off hand |
|---|---|
| `artifacts:bunny_hoppers` | jump strength +40 %, safe fall distance +10 blocks, fall damage multiplier unchanged (upstream default 0) |
| `artifacts:feral_claws` | attack speed +30 % |
| `artifacts:shock_pendant` | lightning damage is cancelled |

A right click with an artifact moves it into an empty off hand (`use_item` on
each item's Part); the off hand is the port's **worn slot**, and an artifact
merely carried in the inventory grants nothing. The attribute modifiers are
re-applied at the end of every server player tick (`player_tick_post`), so
taking an artifact off or disabling it removes them. The Shock Pendant answers
`living_incoming_damage` and cancels a hit whose type is
`minecraft:lightning_bolt` (26.3's whole `#minecraft:is_lightning` tag).
`config/artifacts.conf` holds upstream's defaults.

Upstream equips artifacts in its own slots (Curios on 1.21.1) and only a worn
artifact works; the pin has no custom slot registry, so the off hand is the one
worn slot and each artifact counts once. See [notes.md](notes.md) for the
upstream evidence, the helper → hook table, the deviations, the coverage table
and the verification, and
[`../_shared/upstream/artifacts/FINDINGS.md`](../_shared/upstream/artifacts/FINDINGS.md)
for the source analysis.

**Local-only**: the licence allows derivatives, but the campaign publishes
nothing without the maintainer's approval (ARCHITECTURE.md §8).

## Layout

```text
horizon-mod.toml     the only manifest: identity, api 0.2, side, the native pins
Cargo.toml           one crate; its only dependency is horizon-mod-api from .horizon/sdk/
src/lib.rs           the one #[mod_api::horizon_mod] impl Hooks + the artifact ItemPart
src/runtime.rs       the pure effect core and its 24 unit tests
assets/artifacts/    models, item definitions, textures and lang for the three items
data/artifacts/      item templates and the creative tab
tools/e2e.sh         the in-game scenario: the carried/worn pair, equip, the effects
artifacts.hmod        the delivered package
```

## Build and verify

The SDK layout has no staged tree: every command runs in this directory and
`cargo horizon-mod` fetches `.horizon/sdk/` (gitignored) on first use.

```bash
cd ports/artifacts
CARGO_INCREMENTAL=0 cargo test                                   # 24 passed
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings     # 0 warnings
cargo fmt --check                                                # not --all
rm -f Cargo.lock                                                 # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build                      # -> target/horizon-mod/artifacts.hmod
cp target/horizon-mod/artifacts.hmod ./artifacts.hmod
cargo horizon-mod check artifacts.hmod                           # exit 0
cargo horizon-mod fits artifacts.hmod --build nightly            # "made for", exit 0
sha256sum artifacts.hmod
```

`tools/e2e.sh` needs a launcher-built game carrying the package:

```bash
ports/tools/launcher-game.sh run artifacts            # builds the game, then PASS + EXIT:0
GAME=$CONTROL_GAME bash tools/e2e.sh                  # the same Nightly, no package: EXIT:1
MODE=control bash tools/e2e.sh                        # config off: CONTROL PASS + EXIT:0
SELFTEST=1 bash tools/e2e.sh                          # counterfactual: EXIT:1
```

The in-game legs and their exit statuses are in
[`../_shared/evidence/artifacts-e2e/`](../_shared/evidence/artifacts-e2e/).

The exact results are recorded in [notes.md](notes.md).
