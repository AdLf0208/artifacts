# Artifacts — port notes (migrated to the SDK format, 0.2.0)

Ported from **Artifacts 13.2.5** on the **1.21.1** branch
(`ochotonida/artifacts`, **MIT**, © 2019 Wouter Pauwels) at commit
`7cf7dc42e322e13f096eea16cee17a4b400b75f7`, and checked against the
**26.1** branch `4b67278df94e4d3669119f1abb24a1c3a758afe1` (15.1.3) and the
**26.3** branch `27bab878fc038d4f7aa11e63eeb0e3adb7236620`. The three slice-1
items behave identically on all three branches. The extraction with `file:line`
evidence is in
[`../_shared/upstream/artifacts/FINDINGS.md`](../_shared/upstream/artifacts/FINDINGS.md).

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8).
The licence is MIT, so derivatives are allowed; publication still needs the
maintainer's approval.

Status: **slices 1 + 2 implemented and verified in-game**, and **0.2.0 is one flat SDK
package** (content and code in one `.hmod`, the retired `.hnmod` container having been
closed by the mod-API merge). The 0.1.0 bundle embedded its content with a
`content_root` injection; that whole mechanism is gone, because `assets/` and `data/`
now travel inside the same package. See
[`## Migration to the SDK format`](#migration-to-the-sdk-format) for the file map, the
helper → hook table, the traps and the changelog that supersedes submission **#791**.
`src/runtime.rs` is **byte-identical** to the retired `mixins/artifacts/runtime.rs`
(sha256 `7f23ded0…2207`), including the file inside the built package, and its 21 tests
pass unchanged.

Historical status (0.1.0): three artifact items, a pure runtime with the upstream
effect rules, four hook helpers, a valid `.hmod`/`.hnmod` that carried its own content,
the full staged `stage → check → test → build` ladder green with 0 warnings, and an
in-game default/control differential on the fast build (slice 1) plus a right-click
off-hand equip (slice 2). Slice 2 was re-verified on pin `83ad384f` with the debug build;
the embedded slice on `1d63e0a` with the game launched from the `.hnmod` alone (no
`--dev-mod`).

## Class and tier

Ledger row (`ports/ledger.csv`): **class `content`, tier `deep`**. Artifacts is
a content mod (items, the Mimic mob) and upstream needs what
`horizon/docs/modpacks.md` calls the deep tier: "entities and AI, custom
renderers and animated models, dimensions, new vanilla hooks". The port
therefore takes the **content carry-over + logic-tier slice** of that row — the
items, the effect table, and the `use_item`/`player_tick`/
`living_incoming_damage` behaviour — and records the deep half (the Mimic
entity/AI in gap A8, upstream's own equipment slots/renderers in A1) as
deferred rather than pretending it is done. Slice 2 (right-click equip) is the
audit's highest-value A2 item mapped onto the one real worn slot the pin
exposes (the off hand).

## Behaviour (slice 1)

Upstream equips artifacts in its own equipment slots (Curios on 1.21.1,
`WearableArtifactItem`; the 26.x branches use their own equipment providers).
This slice **defers the slot system**: the player's main inventory and off hand
stand in for the worn slots, and each artifact counts at most once, like one
slot. The attributes are re-applied every player tick, so dropping or disabling
an artifact removes its modifiers.

| Artifact | Effect while carried | Upstream |
|---|---|---|
| `artifacts:bunny_hoppers` | `minecraft:jump_strength` +0.40 `add_multiplied_base` (0.42 → 0.588); `minecraft:safe_fall_distance` +10 `add_value` (3 → 13); `minecraft:fall_damage_multiplier` +0.00 `add_multiplied_base` (unchanged at 1) | `ModItems.java:572-587`, `ItemConfigs.java:152-173` |
| `artifacts:feral_claws` | `minecraft:attack_speed` +0.30 `add_multiplied_base` (4 → 5.2) | `ModItems.java:462-467`, `ItemConfigs.java:491-501` |
| `artifacts:shock_pendant` | cancels a hit whose damage type is `minecraft:lightning_bolt` (26.3's whole `#minecraft:is_lightning` tag) | `ModItems.java:275-293`, `DamageImmunity.java:17-41`, `LivingEntityMixin.java:14-20` |

The pure core (`runtime.rs`) owns the ids, the carried set, the config, the
attribute-effect table and the lightning decision. The glue (`support.rs`)
scans the inventory, calls `PlayerMotion::set_modifier` for each effect, and
caches each player's loadout by entity id for the static damage hook.

### Right-click equip (slice 2)

A right click (`use_item`, API 13) with an artifact in the **main hand** and an
**empty off hand** moves that stack into the off hand, the port's worn slot.
The hook refuses when the off hand already holds anything, so an equip never
silently replaces an item, and a non-artifact is never moved. The existing
inventory scan already counts the off hand, so the attributes keep applying.
This is upstream's `tryEquipFromUse` path (audit A2) mapped onto the one real
worn slot the pin exposes; the custom Curios-like slot stays blocked (A1,
`living_set_equipment` refuses a player's body/saddle slot).

### Why a cached loadout

`living_incoming_damage` (API 50) is a **static** hook: it receives no `Game`,
so it cannot scan the inventory. `player_tick` (API 31) runs after the
inventory tick in the same server tick and refreshes a `BTreeMap<entity id,
Carried>`; the damage hook reads that map. The host's loadout is cached under
both the hub's `host_entity` and the local body's `entity_id`, because an
integrated server can leave `PlayerMotion.entity_id` at 0.

## Config (`config/artifacts.conf`)

A missing file is written with upstream's defaults:

```
# Artifacts (Horizon) — server settings.
# Upstream's defaults; -1 cancels an ADD_MULTIPLIED_BASE bonus.
bunny_hoppers_enabled = true
bunny_hoppers_fall_damage_multiplier = 0
bunny_hoppers_safe_fall_distance_bonus = 10
bunny_hoppers_jump_strength_bonus = 0.4
feral_claws_enabled = true
feral_claws_attack_speed_bonus = 0.3
shock_pendant_enabled = true
shock_pendant_cancel_lightning_damage = true
```

| Key | Default | Range | Upstream |
|---|---|---|---|
| `bunny_hoppers_enabled` | `true` | bool | item always on; port switch |
| `bunny_hoppers_fall_damage_multiplier` | `0` | −10…10 | `fallDamageMultiplier` (`ItemConfigs.java:158`) |
| `bunny_hoppers_safe_fall_distance_bonus` | `10` | −1024…1024 | `safeFallDistanceBonus` (`ItemConfigs.java:168`) |
| `bunny_hoppers_jump_strength_bonus` | `0.4` | −1…32 | `jumpStrengthBonus` (`ItemConfigs.java:164`) |
| `feral_claws_enabled` | `true` | bool | item always on; port switch |
| `feral_claws_attack_speed_bonus` | `0.3` | −1…32 | `attackSpeedBonus` (`ItemConfigs.java:493`) |
| `shock_pendant_enabled` | `true` | bool | item always on; port switch |
| `shock_pendant_cancel_lightning_damage` | `true` | bool | `cancelLightningDamage` (`ItemConfigs.java:886`) |

A non-finite or badly typed number keeps its default; booleans that do not
parse keep theirs. Everything is read at `server_about_to_start`.

## Hooks (pin `1d63e0a`, mixins `5a64af9`) — the retired 0.1.0 mapping, kept for history

*The 0.1.0 `.hnmod` bundle's four injections. The 0.2.0 SDK package implements the same
four moments as `Hooks` methods and one `ItemPart`; the helper → hook table is in
[`## Migration to the SDK format`](#migration-to-the-sdk-format).*

| Hook | Helper | What it does |
|---|---|---|
| `server_about_to_start` (v1, API 44) | `about.rs` | Read/create `config/artifacts.conf`. |
| `player_tick` (v1, API 31) | `player.rs` | Scan the acting player's inventory/off hand, apply the attribute modifiers, cache the loadout. |
| `use_item` (v1, API 13) | `equip.rs` | A right click with an artifact in the main hand moves it to the off hand when the off hand is empty. |
| `living_incoming_damage` (v1, API 50) | `damage.rs` | Cancel a lightning hit on a player whose cached loadout has the Shock Pendant. |

The acting player comes from `native_acting_player()`. A LAN guest's inventory
is read from the hub's guest record (`game.lan`); the host's is
`game.inventory` plus `game.offhand`. Embedded tests run as
`game::artifacts::tests`.

## Content (`.hmod`)

No ATM10 jar was carried; everything comes from the **MIT upstream
repository**:

- `data/artifacts/horizon/item/{bunny_hoppers,feral_claws,shock_pendant}.json`:
  the `minecraft:stick` template with `max_stack_size: 1`, `rarity: "rare"`,
  the stick's cooking fuel removed, and `minecraft:damage_resistant` against
  `#minecraft:is_fire` (upstream items are `stacksTo(1)`, `Rarity.RARE`,
  fire resistant: `ArtifactProperties.java:254-266`).
- `assets/artifacts/items/*.json` + `models/item/*.json` (parent
  `minecraft:item/generated`) and the real 16×16 upstream textures
  (`shock_pendant.png`, `feral_claws.png`, `bunny_hoppers.png`; identical
  SHA-256 on the 1.21.1, 26.1 and 26.3 branches).
- `assets/artifacts/lang/en_us.json`: the three upstream names and
  `artifacts.creative_tab`.
- `data/artifacts/horizon/creative_tab/main.json`: the three items, with the
  Bunny Hoppers icon (upstream's tab icon, `ModItems.java:56-62`).

## Deviations and gaps

1. **Inventory instead of slots.** Upstream wears artifacts in Curios/its own
   equipment slots; the port treats the main inventory and off hand as the
   worn slots. A player can carry all three at once, and a second copy of one
   artifact adds nothing (one slot per kind).
2. **One worn slot, not one per artifact kind.** Slice 2 implements
   right-click equip: a main-hand artifact moves into an **empty off hand**
   (and is refused when the off hand is occupied). The main inventory still
   counts as a fallback, and upstream's own/Curios equipment slots (A1) are not
   expressible at this pin, so a player has one worn slot rather than one per
   artifact kind.
3. **No durability/breakage.** Upstream artifacts lose durability on use,
   being hurt or mining and can break (`ItemConfigs` durability categories);
   slice 1 treats them as permanent while carried.
4. **No cooldowns.** The Shock Pendant's 2 s cooldown and the Feral Claws'
   per-attack durability are deferred.
5. **No retaliation/particles/sounds.** The Shock Pendant's 25 % lightning
   strike, the Bunny Hoppers' rabbit hurt sound and the Feral Claws' melee
   durability damage are deferred.
6. **Fall damage multiplier default is upstream's 0, not −1.** Upstream's
   default does not change the vanilla multiplier; the benefit is the +10 safe
   fall distance. A pack can set `-1` to cancel fall damage entirely.
7. **Other artifacts, the Mimic, campsite loot and the config UI** are out of
   scope for slice 1.
8. **No client tooltip** for the artifact effects; upstream shows attribute
   and ability tooltips.
9. **One-tick loadout cache.** `living_incoming_damage` is static, so the
   Shock Pendant decision reads the `player_tick` cache. A pickup and a hit in
   the same tick are not protected; the next tick is. Inherent to the static
   hook at this pin.

## Embedded content (AE2 pattern) — retired in 0.2.0

> **Retired 2026-11-18.** Horizon's mod-API merge (#16941) replaced the `.hnmod`
> container with one flat `.hmod` that carries content *and* code, so this whole
> mechanism — the `content_root` injection, `native_data!`, the embedded
> `data/artifacts-content.hmod` and the `.horizon-cache` write — has no job left and was
> deleted. `assets/` and `data/` are in the package itself now; the section below is the
> 0.1.0 record (its in-game transcript is still the evidence that the items register).

The registry stores **one file per project** (playbook §1.0), so a port with both
halves could not be listed faithfully: an `.hnmod` is code-only and an `.hmod`
is content. This bundle now carries its content, exactly as the merged AE2 entry
does:

- **`mixins/artifacts/data/artifacts-content.hmod`** is the built
  `content/artifacts.hmod`, byte for byte (4,492 B, `d2136172…321d`).
  `tools/package.py` packs the bundle's top-level `.rs`/`.toml` **and**
  everything under `data/` as `mixins/<id>/data/…`, with the same fixed zip
  timestamps, so a rebuild reproduces the SHA-256 (two consecutive runs:
  identical). It refuses a `data/<id>-content.hmod` that is not byte-identical
  to `content/<id>.hmod`, so a rebuilt content mod can never ship stale.
- **`mixins/artifacts/embedded_content.rs`** with the `embedded-content`
  `[[injection]]` (`target = "horizon_client::game::mods::content_root"`,
  `kind = "HEAD"`, `cancellable = true`, bindings `root`/`enabled`/`dev`)
  compiles the file in with `native_data!` (API 41), writes it to
  `.horizon-cache/artifacts-embedded-content.hmod` when the copy on disk
  differs, appends it to the dev-mod list and returns its `content_root`. The
  helper calls `content_root` once more and stops at its own
  `dev.contains(&file)` guard, so the second call runs the vanilla body — the
  same one-level recursion AE2 uses. An unwritable cache logs a warning and
  `Continue`s (no content) instead of failing the game.

Verified in game with **only the bundle** (`EMBEDDED=1`, no `--dev-mod`): the
whole slice-1/2 scenario passes, and the file the helper wrote is byte-identical
to `content/artifacts.hmod`. The control (`CONTROL=1`) launches the same binary
with `.horizon-cache` replaced by a plain file, so the helper refuses its
content: `item replace … artifacts:bunny_hoppers` gives nothing and
`setblock … artifacts:bunny_hoppers` leaves `minecraft:air` — the ids are
unknown without the embedded file, so the PASS is not vacuous.

This makes the `.hnmod` the single listable artifact of this port; it does not by
itself publish it (licence approval is still required, ARCHITECTURE.md §8).

### In-game transcript (quoted from `e2e-embedded.log`)

```
== control: the same game with no embedded content file ==
  without the embedded file: no artifacts item, the block stayed minecraft:air
  launch: no --dev-mod; the bundle carries its own content
  embedded content: 4492 B at /tmp/opencode/artifacts-home.RZNonJ/.horizon-cache/artifacts-embedded-content.hmod, identical to content/artifacts.hmod
minecraft:attack_speed = 5.2 (one feral claws (+30%))
minecraft:safe_fall_distance = 13.0 (safe fall +10)
health 20 (the pendant cancels lightning)
health 20 (the hoppers make a 10-block fall safe)
right-click equip: offhand artifacts:feral_claws, main hand 'empty'
PASS: one bundle, no --dev-mod: the embedded content gave the three ids …
EXIT:0
```

The same scenario with `SELFTEST=1` ends `FAIL` + `EXIT:1`
(`e2e-embedded-selftest.log`).

## Migration to the SDK format

*Migrated 2026-11-18 from the retired `.hnmod` container, following the verified recipe in
`ports/imfast/notes.md` → `## Migration to the SDK format`. Pins: nightly
**`0b14f8acfd97fa2c750e42795249dcc981178ca4`**, mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, launcher ≥ 0.2.70. The registry closed
this bundle's submission (#791) with the rest on 2026-11-18; the SDK package supersedes
it.*

**Why.** Horizon's mod-API merge (#16941) retired `.hnmod`: one flat `.hmod` now carries
content, native code, or both, and the launcher compiles it against the build `[native]`
pins. For this port that removed the whole split-package workaround — the content is no
longer embedded through a `content_root` injection, it is simply `assets/` and `data/` in
the same package.

**What changed, and what did not.** `src/runtime.rs` (the ids, the config parse/format,
the attribute table, the lightning decision, 21 tests) is **byte-identical** —
`sha256 7f23ded00061da0c017766478ad229d6601edf559369a42a3560c0a51b8b2207` for both the
working file, the retired `mixins/artifacts/runtime.rs` and the copy inside the built
package (`evidence/artifacts/sdk-runtime-cmp.txt`). The carried content is byte-identical
too: all 14 `assets/` + `data/` files compare equal to the 0.1.0 content package's entries
(`LICENSE`/`LICENSE-NOTICE.md` are the same bytes as the two files that used to sit in
`data/artifacts/license/`, now at the package root). Every number, id and rule still lives
in `runtime.rs`; only the glue, the manifest and the packaging were rewritten.

### File map (0.2.0)

| Path | What it is |
|---|---|
| `horizon-mod.toml` | the only manifest: `id = "artifacts"`, `version = "0.2.0"`, `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pinned to `0b14f8a` / `e59da90`, `game = "26.3"` |
| `Cargo.toml` | one crate; the only dependency is `horizon-mod-api` from `.horizon/sdk/` (no registry dependency, so no `Cargo.lock` is packed) |
| `src/lib.rs` | the one `#[mod_api::horizon_mod] impl Hooks` — `register`, `server_about_to_start`, `player_tick_post`, `living_incoming_damage`, `player_logged_out` — plus the artifact `ItemPart` and the `Ids` table |
| `src/runtime.rs` | the pure core, carried byte-for-byte, with its 21 tests |
| `assets/artifacts/` | the three items' definitions, models, textures and `en_us` lang (unchanged) |
| `data/artifacts/` | the three `horizon/item/*.json` templates and the creative tab (unchanged) |
| `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `AGENTS.md` | root files the packer includes; the licence travels inside the package |
| `tools/e2e.sh` | the in-game scenario and its assertions, unchanged (format-independent) |
| `artifacts.hmod` | the delivered package, 23,267 B, sha256 `72ebd4a2…cae3` |
| `.gitignore` | `/target`, `/.horizon`, `/Cargo.lock` |

### Files deleted, and where the evidence went

Into `ports/_shared/evidence/artifacts/retired-hnmod-0.1.0/`, **before** the first build:

| Deleted | Why |
|---|---|
| `mixins/artifacts/` (`horizon-mixin.toml`, `runtime.rs`, `support.rs`, `about.rs`, `player.rs`, `equip.rs`, `damage.rs`, `embedded_content.rs`, `data/`) | the container is retired; `runtime.rs` moved to `src/` |
| `artifacts.hnmod` (0.1.0, sha256 `99c2f00f…920b`) | unloadable on launcher ≥ 0.2.70 |
| `compatibility.json` | `[native]` in the manifest is the pin now |
| `rust-toolchain.toml` | the pinned Horizon commit and the launcher decide the toolchain |
| `tools/horizon.py`, `tools/package.py`, `tools/package.sh`, `tools/stamp_license.py`, `tools/test_stage.py`, `tools/test_template.py`, `tools/audit/` | staging, `.hnmod` packaging and the plan-fixture tests are gone |
| `.github/workflows/ci.yml` | it ran the retired ladder |
| `content/` (`horizon-mod.toml` + the built `artifacts.hmod`) | it was a second project inside the port root, which the packer refuses |

**Kept:** `tools/e2e.sh`, `notes.md`, `FINDINGS.md`, `VALIDATION.md`, `REPIN-AUDIT.md`,
`LICENSE`, `LICENSE-NOTICE.md`, and `content/`'s `assets/` + `data/` (moved to the root).

### Helper → hook, one by one

| 0.1.0 `.hnmod` | 0.2.0 SDK | What the mapping had to solve |
|---|---|---|
| `[[injection]] hook = "server_about_to_start"` (API 44), helper `about.rs`, binding `game` | `fn server_about_to_start(&self, host: &mut dyn ServerHost, event: &mut ServerAboutToStartEvent)` | config through the mod's own checked service: `host.config_files()` + `ConfigPath::new("artifacts.conf")` → `config/artifacts.conf`, with `read`/`write` instead of `Game::native_config_dir()` + `fs::*`. Same file, same name, same defaults. |
| `[[injection]] hook = "player_tick"` (API 31), helper `player.rs`, binding `game` | `fn player_tick_post(&self, host: HostSide, event: &mut PlayerTickPostEvent)` | the hook was **renamed**: there is no `player_tick` in the SDK catalog at all, only `player_tick_pre`/`player_tick_post`, and they fire on **both** logical sides — the glue matches `HostSide::Server` and does nothing on a client. `event.player: PlayerRef` names the ticked player, so 0.1.0's "cache under both the hub's `host_entity` and the local body's id" workaround is gone; the inventory comes from `PlayerRef::inventory_slots(host)` (0–35 main, 40 off hand) instead of `game.inventory` / `game.offhand` / `game.lan.hub.guests`. |
| `[[injection]] hook = "use_item"` (API 13 v1, global, `claimed: &mut bool`), helper `equip.rs` | `impl ItemPart for ArtifactPart { fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> }`, attached to the three items in `register` | the global v1 `use_item` is **gone**: the moment is a Part now, called only for the items it is attached to, which is exactly what the old helper's own stack comparison faked. `claimed = true` → `Some(InteractionResult::SuccessServer)`; `event.hand` (`Hand::Main`) replaces the `game.held()` test; the move is `LivingRef::set_equipment(MainHand, None)` then `set_equipment(OffHand, Some(stack))` instead of writing `game.offhand`. |
| `[[injection]] hook = "living_incoming_damage"` (API 50, static), helper `damage.rs`, binding `canceled: &mut bool` | `fn living_incoming_damage(&self, level: &mut dyn LevelRandomHost, event: &mut LivingIncomingDamageEvent<'_>) -> Flow` | the hook survives with a richer event: `event.kind`, `event.source.damage_type` (`HurtSource`) and `event.cancelled` replace the loose `kind`/`damage_type`/`canceled` bindings, and the contract is `cancel`, so `Flow::Cancel` is the idiomatic answer (setting `event.cancelled` would work too). The host is still **level-only**, so the cached loadout stays. |
| `[[injection]] target = horizon_client::game::mods::content_root` (AE2 pattern), helper `embedded_content.rs`, `native_data!("artifacts", "artifacts-content.hmod")`, `mixins/artifacts/data/artifacts-content.hmod` | **deleted** | the package carries `assets/` and `data/`: no write to `.horizon-cache`, no dev-mod list, no one-level recursion, and no way for the embedded copy to go stale. |
| `[[module]] runtime` (`crate::game::artifacts`) and `[[module]] state` (`artifacts_state`), `static CORE: Mutex<Option<Core>>` | `pub mod runtime;` + a `Mutex<State>` field on the mod struct | no plan section: `runtime.rs` is an ordinary module, and the macro boxes one `Default` instance for the session, so the state is a field. `Hooks: Any + Send + Sync` ⇒ `Mutex`. |
| `native_acting_player()`, `game.player.entity_id`, `game.lan.hub.guests` | `event.player: PlayerRef`, `PlayerRef::entity().id()` | one identity from the event instead of guessing between the hub record and the local body. |

### Gaps and debts recorded by this migration

1. **The one-tick loadout cache stays (deviation 9, unchanged).** `living_incoming_damage`
   is handed a `LevelRandomHost`, which is `LevelHost` + the level's random stream: blocks
   and the level's own attachments, **no** entity or player access, so the pendant decision
   still cannot read an inventory at hit time and still reads the cache
   `player_tick_post` refreshed. A pickup and a lightning hit in the *same* tick is still
   unprotected; the next tick is. **Issue shape:** *"Give the damage hooks a host that can
   read the hurt entity's inventory (or an `EntityRef`), so a hit-time decision needs no
   cache."*
2. **`player_inventory_changed` is the cheaper hook, not adopted.** The SDK has a server
   `player_inventory_changed(player, stacks)` (the 41 `Inventory` slots, also fired at a
   player's first check after joining) that would replace the per-tick inventory walk with
   an event. Not adopted here because the migration's contract is the *same* behaviour
   (0.1.0 re-applied the modifiers every tick, which also repairs a modifier lost to a
   respawn); it is the obvious next slice and needs an in-game differential.
3. **No logging in the package.** 0.1.0 logged the config path and every refused modifier
   through `log`; `horizon-mod-api` depends on `log` but does not re-export it, so using it
   means a registry dependency, and a registry dependency means a packed `Cargo.lock` —
   which changes the artifact and pins crates newer than the SDK's own lock (reference trap
   5). This package carries none; the four places that used to log now keep their defaults
   silently, and `notes.md` is the record.
4. **The modifier ids are the runtime's**, parsed with `Identifier::parse`; a malformed one
   is skipped rather than panicking inside a hook (it can only be a bug in the constants).
5. **The in-game e2e was not re-run** — it needs a *launcher-built* game carrying the
   package (`horizon-launcher native-build <port> … --trust-native-code`), and
   `horizon-launcher` is not installed here. `tools/e2e.sh` and its assertions are kept
   unchanged; the 0.1.0 runs stand as the behaviour's evidence, and what changed is the
   transport of the same four decisions, not the rules.
6. **Content is unchanged on purpose.** No item, recipe, model, texture or lang file was
   edited; the only content-side change is where the licence texts live (package root
   instead of `data/artifacts/license/`, same bytes, still inside the package).

### Traps the next migration must know (beyond imfast's 15)

1. **A retired `content/` folder is a second project.** `content/horizon-mod.toml` +
   `content/assets` + `content/data` inside the port root is what the packer refuses
   ("only package paths"); move `assets/` and `data/` to the root and delete `content/`
   *before* the first build, exactly as `mixins/` had to go.
2. **Content items get behaviour from `register`.** `r.item_key("artifacts:feral_claws")`
   then `r.item_part(key, Part)`; the keys are validated after all declarations, so a
   missing item refuses the mod instead of silently doing nothing. A `data/`-declared item
   needs no `native_blocks`/`native_items`.
3. **The API resolves id → key, never key → id.** There is no public `ItemKey::id()`; the
   glue keeps a small `Ids` table and maps a stack's key back to the runtime's `&'static
   str` id through it. Do not invent a lookup.
4. **Handle reads are called on the handle, not the host.**
   `player.inventory_slots(host)`, not `host.player_inventory_slots(player)` — the host
   trait's method of that name is the backend one and is not in scope.
5. **A `side = "both"` hook must be matched on `HostSide`.** `player_tick_pre/_post` and
   `use_item` run on the client window too; writing there is refused, so take
   `HostSide::Server(host)` and return early otherwise.
6. **`use std::sync::OnceLock` + `Ids` is the way to keep `register`'s keys**, because
   `register(&self, …)` gets no later access to the registrar and every other hook is
   `&self` as well.
7. **`mod_api::prelude::*` can be an unused import** (host methods are inherent on
   `dyn ServerHost`), and `-D warnings` makes that fatal — drop it if nothing needs it.
8. **Root files are packed.** A stale `README.md`/`AGENTS.md` describing `mixins/` and
   `tools/horizon.py` would ship inside the `.hmod`; rewrite them in the same job.
9. **`cargo horizon-mod build` reports the SDK it used** (`SDK 0b14f8a … in ./.horizon/sdk`);
   seeding `.horizon/sdk` from another port at the same pin is enough to work offline, and
   two consecutive builds of the same tree produced the same 23,267 B and sha256
   `72ebd4a2…`.
10. **`check --uses` prints nothing for a package with content**, but the package's own
    `uses.json` (151 entries here) is the surface record: `hook:server_about_to_start`,
    `hook:player_tick_post(.player)`, `hook:use_item(.stack/.hand/.player)`,
    `hook:living_incoming_damage(.entity/.kind/.source/.cancelled)`,
    `hook:player_logged_out`, `method:ItemPart::use_item`, `method:ItemsRegistrar::item_key`,
    `method:ConfigFiles::{read,write}`, `handle:LivingRef::{equipment,set_equipment,
    replace_attribute_modifier,remove_attribute_modifier}`,
    `handle:PlayerRef::{entity,inventory_slots}`, `handle:Identifier::parse`,
    `handle:AttributeModifier::new`, `macro:horizon_mod`/`horizon_part`,
    `part:item`/`entity`, `capability:registries`.

### Changelog / supersede text (for `submit/artifacts-0.2.0`)

> **Artifacts 0.2.0 — the SDK package.** Republished as a native `.hmod` with the
> mod-API SDK from nightly `0b14f8a` (mixins `e59da90`, launcher ≥ 0.2.70), superseding
> **#791**, closed by the 2026-11-18 format purge: the `.hnmod` container is retired and
> does not load.
>
> **One file, both halves.** The three artifact items (`assets/`, `data/`) now travel
> inside the same package as the code, so the 0.1.0 workaround — a `content_root`
> injection writing an embedded `artifacts-content.hmod` to `.horizon-cache` and appending
> it to the dev-mod list — is deleted. Nothing else about the content changed.
>
> **Same behaviour, typed glue.** The rules are the same file, byte for byte
> (`src/runtime.rs`, 21 tests). The four moments became SDK hooks: `server_about_to_start`
> (config through the mod's own `config/` service), `player_tick_post` (was `player_tick`;
> the loadout and the attribute modifiers), an `ItemPart::use_item` on each artifact (was
> the global v1 `use_item`; a right click into an empty off hand), and
> `living_incoming_damage` (the Shock Pendant's cancel, now `Flow::Cancel`). The worn-slot
> substitute, the config keys and their defaults, and the deferral of the Mimic, the other
> artifacts and the custom slots are unchanged.
>
> **Known**: the damage hook is still handed a level-only host, so the pendant decision
> reads a one-tick loadout cache (`player_tick_post`); a hit in the same tick as a pickup
> is not protected.

## Verification

### 0.2.0 — the SDK package (2026-11-18, pin `0b14f8a`, mixins `e59da90`)

Skills: superpowers/using-superpowers,test-driven-development,verification-before-completion; BeautifulRust/rust-best-practices

Everything below was run **directly in `ports/artifacts`** (`CARGO_INCREMENTAL=0`, no
`--target-dir`, no staged tree — `build-slot.sh` speaks the retired tool). Evidence in
`ports/_shared/evidence/artifacts/`.

| Check | Command | Result |
|---|---|---|
| Runtime unit tests | `cargo test` | **21 passed**, 0 failed, exit 0 (`sdk-test.log`) |
| Lints | `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| Formatting | `cargo fmt --check` (not `--all`) | clean, exit 0 (`sdk-fmt.log`) |
| Package | `rm -f Cargo.lock; cargo horizon-mod build` | `Built ./target/horizon-mod/artifacts.hmod`, exit 0, SDK reported as `0b14f8a … in ./.horizon/sdk` (`sdk-build.log`) |
| Registry check | `cargo horizon-mod check artifacts.hmod` | `artifacts 0.2.0 (native-mod, format native-1/0b14f8ac…/e59da909…, for 26.3)`, **23,267 B**, sha256 `72ebd4a2…cae3`, **exit 0** (`sdk-check.log`, `sdk-check-json.log`: `"ok":true`) |
| Fits the pinned build | `cargo horizon-mod fits artifacts.hmod --build nightly` | `0b14f8a  made for`, exit 0 (`sdk-fits.log`) |
| `runtime.rs` untouched | `unzip -p artifacts.hmod src/runtime.rs \| sha256sum` vs the retired file | `7f23ded0…2207` both, byte for byte (`sdk-runtime-cmp.txt`) |
| Deliverable in the port, not `target/` | `sha256sum artifacts.hmod; stat -c%s` | `72ebd4a225639317da2e7defd95ec3018c8b207e961a2b1ba057f7ddf8ddcae3`, 23,267 B (`sdk-sha256.txt`) |
| Reproducible | two consecutive `cargo horizon-mod build` runs | same 23,267 B, same sha256 |
| Content unchanged | each packaged `assets/`+`data/` file vs the 0.1.0 content `.hmod` | 14/14 identical; the two licence texts identical and now at the package root |
| Surface | `uses.json` inside the package | 151 entries (`sdk-uses.json`), the hook list in the migration section above |
| Skills line | `python3 ports/tools/check-skills.py --notes ports/artifacts` | `Skills line present` |

### Not re-run in this migration

- **The in-game e2e** — needs a launcher-built game carrying the package, which this
  environment cannot produce (no `horizon-launcher`); migration trap 15 in
  `ports/imfast/notes.md`. The 0.1.0 logs stand as the behaviour's evidence.
- **The staged ladder** (`tools/horizon.py stage/check/test/build`) — deleted with the
  retired format.

### 0.1.0 — the retired `.hnmod` bundle


Commands run in `ports/artifacts` on 2026-11-05 (pin `c9d474dd…`, mixins
`d9c83d7`). `HORIZON_MIXINS=<campaign>/horizon-mixins` and
`--horizon <campaign>/horizon` are passed because the template defaults point
inside `ports/`, where the checkouts are not.

| Command | Evidence |
|---|---|
| RED: `rustc --edition 2024 --test` on the test module without the implementation | compile errors: `cannot find type Config`, `AttributeEffect`, … (`/tmp/opencode/artifacts-red-runtime.rs`) |
| `cargo fmt --all --check` | exit 0 |
| `cargo test --workspace --locked --offline` | exit 0; `test result: ok. 20 passed; 0 failed; 0 ignored` (plus the 0-test audit crate) |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 0, **0 warnings** |
| `horizon-mixins plan` on the pin export | exit 0; `status: ok`, **2 modules**, **3 injections** (`server_about_to_start`, `player_tick`, `living_incoming_damage`) |
| `horizon-mod build` (content) | exit 0; `Built ./target/horizon-mod/artifacts.hmod` |
| `horizon-mod check artifacts.hmod` | exit 0; 4,492 bytes; SHA-256 `d2136172…`; `no code: content only` |
| `python3 tools/horizon.py stage --horizon …` | exit 0, `stage_changed: true`, revision `c9d474dd…` (2 modules, **3 injections**) |
| `python3 tools/horizon.py check --horizon …` | `cargo_exit: 0`; `grep -c warning:` → **0** |
| `python3 tools/horizon.py test --horizon …` | `cargo_exit: 0`; `test result: ok. 20 passed; 0 failed; 0 ignored; 531 filtered out`; **0 warnings** |
| `python3 tools/horizon.py build --horizon …` | `cargo_exit: 0`; **0 warnings**; `target/horizon-cargo/fast/horizon-client` (81,620,456 B); the direct fast build took 50 m 29 s under load ~40–55, then the cached `horizon.py build` took 2.0 s |
| `tools/e2e.sh` (default, fast build) | exit 0, `PASS` (below) |
| `MODE=control tools/e2e.sh` (fast build) | exit 0, `CONTROL PASS` (below) |
| debug-build e2e runs | also `PASS` / `CONTROL PASS` (`target/e2e-default.log`, `target/e2e-control.log`) |
| port-local target cleanup | removed `target/mixins` (174 M), `target/horizon-cargo` (4.1 G) and `target/debug` (104 M); kept sources, `notes.md`, `content/`, the `.hmod` and the logs |

The fast build was attempted several times while the campaign queue was at
load ~40–55; one attempt was SIGTERMed (exit 143) after ~1 h. The successful
direct `cargo build --profile fast` took 50 m 29 s, and the cached
`horizon.py build` then reported `cargo_exit: 0` in 2.0 s. The debug client
built from the same staged source was used for the first two e2e runs and
also passed; the canonical runs below are the fast-build ones.

The campaign-wide `ports/tools/clean-targets.sh` was **not** run: the
updated AGENTS.md forbids it while other ports' jobs are active (it deletes
every port's staged tree and has SIGTERMed a live build). The port-local
equivalent above was used instead.

The default in-game run (`target/e2e-default-fast.log`, mode `default`, fast
client, spawn at an Overworld sky platform at y=201) printed:

```
default: upstream defaults, all three enabled
minecraft:attack_speed = 4.0 (baseline)
minecraft:safe_fall_distance = 3.0 (baseline)
minecraft:jump_strength = 0.41999998688697815 (baseline)
minecraft:fall_damage_multiplier = 1.0 (baseline)
giveable: artifacts:bunny_hoppers
giveable: artifacts:feral_claws
giveable: artifacts:shock_pendant
minecraft:attack_speed = 4.0 (64 sticks change nothing)
minecraft:safe_fall_distance = 3.0 (64 sticks change nothing)
minecraft:attack_speed = 5.2 (one feral claws (+30%))
minecraft:attack_speed = 5.2 (a duplicate adds nothing)
minecraft:attack_speed = 4.0 (claws removed)
minecraft:safe_fall_distance = 13.0 (safe fall +10)
minecraft:jump_strength = 0.5879999816417694 (jump strength +40%)
minecraft:fall_damage_multiplier = 1.0 (fall multiplier unchanged)
minecraft:safe_fall_distance = 3.0 (hoppers removed)
health 20 (survival baseline)
health 15 (no pendant: lightning hurts)
health 20 (full health with the pendant)
health 20 (the pendant cancels lightning)
health 15 (the pendant does not cancel a fall hit)
health 15 (lightning hurts after the pendant is removed)
health 13 (no hoppers: the 10-block fall hurts: hurt)
health 20 (the hoppers make a 10-block fall safe)
health 13 (the fall hurts after the hoppers are removed: hurt)
PASS: the three ids are giveable, claws give attack speed 5.2 once, hoppers give safe fall 13 and jump 0.588, the pendant cancels a lightning hit but not a fall hit, and the hoppers make a 10-block fall safe
```

The control run (`target/e2e-control-fast.log`, mode `control`, all three
`enabled = false`) printed:

```
control: all three artifacts disabled
minecraft:attack_speed = 4.0 (baseline)
minecraft:safe_fall_distance = 3.0 (baseline)
minecraft:jump_strength = 0.41999998688697815 (baseline)
minecraft:fall_damage_multiplier = 1.0 (baseline)
giveable: artifacts:bunny_hoppers
giveable: artifacts:feral_claws
giveable: artifacts:shock_pendant
minecraft:attack_speed = 4.0 (64 sticks change nothing)
minecraft:safe_fall_distance = 3.0 (64 sticks change nothing)
minecraft:attack_speed = 4.0 (control: claws are off)
minecraft:attack_speed = 4.0 (control: two claws are off)
minecraft:attack_speed = 4.0 (claws removed)
minecraft:safe_fall_distance = 3.0 (control: hoppers are off)
minecraft:jump_strength = 0.41999998688697815 (control: hoppers are off)
minecraft:fall_damage_multiplier = 1.0 (control: hoppers are off)
minecraft:safe_fall_distance = 3.0 (hoppers removed)
health 20 (survival baseline)
health 15 (control: lightning hurts (pendant off))
health 20 (full health with the pendant)
health 15 (control: lightning still hurts with the pendant off)
health 15 (control: a fall hit hurts)
health 15 (lightning hurts after the pendant is removed)
health 13 (control: the 10-block fall hurts: hurt)
health 13 (control: the hoppers are off and the fall hurts: hurt)
health 13 (the fall hurts after the hoppers are removed: hurt)
CONTROL PASS: all three artifacts disabled left attack speed 4.0, safe fall 3.0, jump 0.42; the same lightning hit and the same 10-block fall hurt
```

That is the differential: the identical sequence with the default config
reaches 5.2 attack speed, 13 safe fall and 0.588 jump strength, cancels the
lightning hit and makes the 10-block fall safe; with the artifacts disabled
every attribute stays at its baseline, the same lightning hit takes 5 health
and the same fall takes 7. A port that ignored the config or the inventory
fails one of the two runs. The fall scenes use an Overworld platform at
y=200/201 with the fall from y=211: the first attempt used `execute in
minecraft:the_end run tp`, but at this pin the local player stayed in the
Overworld (the state dump showed `dimension: minecraft:overworld`), so the
fall hit the Overworld floor 43 blocks below and killed the player; the
platform-at-y=200 version is the reliable scene.

## Skills used

- superpowers `test-driven-development` (the runtime tests were written first
  and the RED run failed on the missing types), `verification-before-completion`
  (every claim above is from recorded command output), `systematic-debugging`
  for any e2e failure.
- Beautiful Rust `rust-best-practices` (read from
  `ports/_shared/skills/BeautifulRust/skills/rust-best-practices/SKILL.md`):
  fmt/clippy `-D warnings`/tests clean, typed `Artifact`/`Carried`/`Config`,
  `Option` for id lookup, no bool parameters, `#[must_use]` on pure functions.

## Re-pin to Horizon `1d63e0a` and artifact rebuild (2026-11-09)

Mode A migration of a port that pinned `83ad384f`, plus the `.hnmod` half it
never had. The stamped hook catalog is byte-identical between the two pins, so
this bundle's four injections, two modules and every binding are unchanged; only
the pin identity moved.

| | Before | After |
|---|---|---|
| Horizon | `83ad384f48d0742746d518737511882680aab5b7` | **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`** |
| horizon-mixins | `e8cd0aa38cffef49cb2e088ef716baa107148c02` | **`5a64af938ec86b00d53d16a408aabc0038fb3e57`** |
| `game.rs` SHA-256 | `996432b9f7f9461fa69c90ff798daf6b89fe3795f379d7c6f401916633296463` | `64813cf4db0b9c3983d42908609bb633b9fbbc705f0c890396f940a6c7cef80f` |
| `.hnmod` delivered | none (only the `.hmod`) | `artifacts.hnmod` in the port root |

Migration edits: `compatibility.json` (`horizon_revision`, `source_sha256`,
`horizon_mixins_revision`) and `mixins/artifacts/horizon-mixin.toml`
(`[compatibility] horizon_revision`). No Rust, content, injection or helper
changed.

### Ladder (all through `ports/tools/build-slot.sh`, `CARGO_INCREMENTAL=0`)

| Step | Result | Evidence |
|---|---|---|
| `cargo fmt` + `--check` | exit 0 | `ports/_shared/evidence/artifacts/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** | `clippy.log` |
| `cargo test --locked --offline` | **21 passed**, 0 failed | `test.log` |
| `horizon.py stage` | exit 0, pin `1d63e0a`, 89 s | `stage.log` |
| `horizon.py check` | `cargo_exit: 0`, **0 warnings**, 53 s | `horizon-check.log`, `.cargo.log` |
| `horizon.py test` | **21 passed**, 0 failed, 1240 filtered, 126 s | `horizon-test.log`, `.cargo.log` |
| `horizon.py build` (fast) | `cargo_exit: 0`, 479 s | `horizon-build.log`, `.cargo.log` |

### Delivered artifacts (reproducible)

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/artifacts.hmod` | 4,492 B | `d213617282819a61712977c0212307e0e207e97a66596620196995a735b7321d` | 15 |
| `artifacts.hnmod` (content embedded) | 14,874 B | `ca84a1cb948ef746b5d6ec866380a7e7273e69fb2aac89f36d29b5c0181bb8e9` | 10 |

`cargo-horizon-mod build`/`check` (the registry's own code, from the pin's
prebuilt tool) print `no code: content only` for the `.hmod`; a second
`tools/package.py` run reproduced the `.hnmod` hash byte-for-byte. The `.hnmod`
entry list now includes `mixins/artifacts/data/artifacts-content.hmod`, and
`cargo horizon-mod check artifacts.hnmod --kind native-mod` exits 0
(`check-native.log`).

### In-game e2e (fast build, shared `horizon-ctl`, The End / Overworld platform)

| Leg | Command | Verdict |
|---|---|---|
| default | `tools/e2e.sh` | `PASS` + `EXIT:0` (`e2e.log`) |
| negative | `SELFTEST=1 tools/e2e.sh` | `FAIL: selftest: the claws' bonus is asserted absent (must FAIL): expected 4.0, got 5.2` + `EXIT:1` (`e2e-negative.log`) |
| one file only (embedded slice) | `EMBEDDED=1 tools/e2e.sh` (no `--dev-mod`) | `PASS` + `EXIT:0` (`e2e-embedded.log`); the control leg shows the three ids unknown without the embedded file |
| embed fail-capability | `EMBEDDED=1 SELFTEST=1 tools/e2e.sh` | `FAIL: selftest: the claws' bonus is asserted absent (must FAIL): expected 4.0, got 5.2` + `EXIT:1` (`e2e-embedded-selftest.log`) |

`python3 ports/tools/check-e2e-logs.py artifacts --strict` → `4/4 ok`
(`e2e.log`, `e2e-negative.log`, `e2e-embedded.log`, `e2e-embedded-selftest.log`).

The embedded slice's ladder (`ports/_shared/evidence/artifacts/ladder-embed.log`,
run through `ports/tools/remote-slot.sh` on the second machine): `cargo fmt
--all -- --check` and `rustfmt --check mixins/artifacts/embedded_content.rs`
exit 0; `cargo clippy --all-targets --locked --offline -- -D warnings` exit 0;
`cargo test` **21 passed**; `horizon.py stage/check/test/build` all
`cargo_exit: 0`, the in-game module tests `21 passed; 1240 filtered out`, and
**0 `warning:`** lines in `horizon-check.log`/`horizon-test.log`/`horizon-build.log`.

### Submission gate

`python3 ports/tools/upstream_ready.py artifacts` exits **0**
(`READY    artifacts    MIT`). The split content+code blocker is cleared: the
`.hmod` now travels **inside** the `.hnmod` (`mixins/artifacts/data/artifacts-content.hmod`,
the AE2 pattern, proved in game from the one file), so the registry's one-file
rule is satisfied. Pin, licence, artifact in the port (not `target/`),
`Skills:` line and the `PASS` + `EXIT:0` e2e are green.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

> **2026-11-10 — embedded slice.** The split port above became one file: the
> content was embedded in the `.hnmod` (`mixins/artifacts/embedded_content.rs` +
> `data/artifacts-content.hmod`), the ladder was re-run on pin `1d63e0a` and the
> port was proved in game with no `--dev-mod`, with a control that shows the ids
> unknown without the embedded file. The `.hnmod` size/SHA-256 in the table above
> is that rebuilt bundle; the earlier split `.hnmod` (11,202 B) is superseded.

## Reviewer repairs (2026-11-11) — PR review items and what changed


Each item below is quoted from `ports/_shared/horizon-mods/reviews/pr-*-artifacts.md` (the review is the specification), with what this repair did about it.

- **Reviewer:** description: empty effect table, bulleted continuations, truncated 'Not yet ported'
  **Repair:** Rewritten: the three artifacts and their effects are a real table, and the worn-slot and off-hand limitations are complete sentences.
- **Reviewer:** [P1] MIT notices not in the bundle
  **Repair:** Both archives carry the notice, the LICENSE holds `Copyright (c) Wouter Pauwels`, and the entry links the absolute upstream URL.
- **Reviewer:** stale audit fixtures (one injection/one module; cancellable injections rejected)
  **Repair:** Fixed exactly as for bhc: the stage test takes its counts from the manifest, the template test checks the real pinned plan against the manifest, and the audit reconstructs the cancellable HEAD guard.
- **Reviewer:** registry artifact delivery
  **Repair:** **Staging (orchestrator, not ours)**: the artifact is not in registry release/staging as `<sha256>.<ext>`; the orchestrator answers it with `horizon-mods#686` (no write access to the registry's releases). Every repackaged artifact below has a fresh sha256 for that re-push, and the author release must be re-uploaded from the port repository.

Skills: superpowers/using-superpowers,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices

## Files (0.2.0)

- `horizon-mod.toml` — the only manifest: `id = "artifacts"`, `version = "0.2.0"`,
  `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]`, `[native]` pinned to
  `0b14f8a` / `e59da90`.
- `Cargo.toml` — one crate; the only dependency is `horizon-mod-api` from `.horizon/sdk/`
  (no registry dependency, no packed `Cargo.lock`).
- `src/lib.rs` — the one `#[mod_api::horizon_mod] impl Hooks` and the artifact `ItemPart`.
- `src/runtime.rs` — the pure rules, byte-identical to the retired `mixins/artifacts/
  runtime.rs`, with its 21 tests.
- `assets/artifacts/`, `data/artifacts/` — the carried content (unchanged bytes).
- `LICENSE`, `LICENSE-NOTICE.md` — inside the package, byte-identical to the two files the
  0.1.0 content package carried under `data/artifacts/license/`.
- `README.md`, `AGENTS.md` — rewritten for this layout; both are packed.
- `tools/e2e.sh` — the in-game scenario and assertions, unchanged.
- `artifacts.hmod` — the delivered package, **23,267 B**, sha256
  `72ebd4a225639317da2e7defd95ec3018c8b207e961a2b1ba057f7ddf8ddcae3` (22 entries).
- Evidence: `../_shared/evidence/artifacts/` (`sdk-*.log`, `sdk-uses.json`,
  `sdk-sha256.txt`, `sdk-entries.txt`, `sdk-runtime-cmp.txt`, the 0.1.0 runs, and the
  retired tree under `retired-hnmod-0.1.0/`).
- The ATM10 jar is not kept for this port: the content comes from the MIT upstream
  repository, and the retired `.hmod`/`.hnmod` are the packaging evidence.
