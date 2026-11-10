# artifacts (Horizon port)

A Horizon port of [artifacts](ochotonida/artifacts) — MIT.

**Status: first slices.** What is below is what the port actually does today.

## Implemented so far

- Upstream equips artifacts in its own equipment slots (Curios on 1.21.1,
- `WearableArtifactItem`; the 26.x branches use their own equipment providers).
- This slice **defers the slot system**: the player's main inventory and off hand
- stand in for the worn slots, and each artifact counts at most once, like one
- slot. The attributes are re-applied every player tick, so dropping or disabling
- an artifact removes its modifiers.
- | Artifact | Effect while carried | Upstream |
- |---|---|---|

## Not yet ported

- Inventory instead of slots.** Upstream wears artifacts in Curios/its own
- equipment slots; the port treats the main inventory and off hand as the
- worn slots. A player can carry all three at once, and a second copy of one
- artifact adds nothing (one slot per kind).
- One worn slot, not one per artifact kind.** Slice 2 implements
- right-click equip: a main-hand artifact moves into an **empty off hand**
- (and is refused when the off hand is occupied). The main inventory still
- counts as a fallback, and upstream's own/Curios equipment slots (A1) are not

## Install

Build the bundle with a native-mod capable launcher, or add the file from the
registry once it is listed. Pinned to Horizon `1d63e0a` and horizon-mixins
`5a64af93` for game 26.3.

## Build from source

```sh
cargo test
python3 tools/horizon.py check --horizon <horizon checkout>
python3 tools/horizon.py build --horizon <horizon checkout>
python3 tools/package.py
```

## Credits and licence

Original mod: ochotonida/artifacts. Port: AdLf0208. Licence: MIT — see [LICENSE](LICENSE);
the original notices are preserved.
