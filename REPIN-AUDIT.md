# Artifacts — re-pin audit against Horizon `83ad384f`

Read-only audit, 2026-11-05. **No port source, content, manifest or tool was
changed; no build, test or e2e was run.** Every symbol below was read from the
new pin export, not inferred.

| | Old | New |
|---|---|---|
| Horizon | `c9d474dd9116e84c243f8992d75c11314df56f06` (108 hooks) | **`83ad384f48d0742746d518737511882680aab5b7`** (427 hooks, 159 helpers, `NATIVE_HOOKS_API = 409`) |
| horizon-mixins | `d9c83d7096b7ea5b83678da2efdfd6161f74997e` | **`e8cd0aa38cffef49cb2e088ef716baa107148c02`** |
| plan schema | 6 | 6 (unchanged) |

Sources: `ports/_shared/pins/horizon-83ad384f/native-hooks.json`,
`crates/horizon-client/src/**`, `crates/horizon-world/src/**`,
`docs/native-mods.md`, `docs/mods.md`, `crates/horizon-mods/src/content.rs`;
this port's `notes.md` and `../_shared/upstream/artifacts/FINDINGS.md`.

Classification used below: **usable** = a writable/cancellable hook or a
helper exists; **hook-only** = only a listen hook; **absent** = no hook/helper.

Method: each deferral in `notes.md`/`FINDINGS.md` was mapped to
`native-hooks.json` and the `horizon-client`/`horizon-world` helpers;
content-only items were checked in `horizon-mods`/`docs/mods.md`. The brief
states main `696d0216` has the same hook set; this audit uses the pinned
export.

## Re-pin safety (existing injections)

Every injection this port already uses is still present at the new pin. The
`horizon.py` wrapper needs the campaign's re-pin tool fixes first, or `test`
runs zero tests vacuously.

| Existing hook | New-pin signature | Verdict |
|---|---|---|
| `server_about_to_start` | `native_server_about_to_start(&mut self, dedicated: bool)` — `native_hooks.rs:2404` | unchanged; the `dedicated` parameter is unbound and fine |
| `player_tick` | `native_player_tick(&mut self)` — `native_hooks.rs:2547` | unchanged |
| `living_incoming_damage` | `native_hooks.rs:7709`; gained `causing`, `direct`, `amount: &mut f32` | bindings still resolve; the helper may now also scale `amount` |

Re-pin steps from [`../_shared/repin-83ad384f.md`](../_shared/repin-83ad384f.md):
`compatibility.json` → new horizon/mixins revisions and
`game.rs` SHA-256 `996432b9f7f9461fa69c90ff798daf6b89fe3795f379d7c6f401916633296463`;
manifest `[compatibility] horizon_revision` must match; `tools/horizon.py:121-122`
must switch the embedded-test target from `-p horizon-client --bin
horizon-client` to `--lib` (the old form runs 0 tests); `horizon-ctl` is now
`-p horizon-client-lib --bin horizon-ctl`; one online `cargo fetch --locked`
is needed for the new pin's `zune-jpeg`. This audit does not claim those steps
were executed.

## Deferred features

| ID | Deferred feature (notes) | Best new-pin API (symbol, file:line) | Class | Verdict | Next step |
|---|---|---|---|---|---|
| A1 | Worn equipment slots instead of inventory/off hand (`notes.md:115-118`, `FINDINGS.md:57-61`) | `native_living_equipment` `native_living_refs.rs:73`; `native_living_set_equipment` `native_living_refs.rs:122`; `native_on_equip` `native_hooks.rs:7623`; API 382/39 `docs/native-mods.md:1648,326` | usable (vanilla slots only); **no custom slot registry** | **partial** — main/off hand/head/chest/legs/feet can be read and written; players have no body/saddle slot (`native_living_refs.rs:113-119`), so there is no Curios-like artifact slot | Target off hand (and optionally one armour slot) for the “worn” set; keep the inventory scan for overflow. Record the custom-slot form as an API request, do not pretend it is solved. |
| A2 | Right-click equip (`notes.md:119-120`) | `native_use_item` `native_hooks.rs:6523` (claim); `native_living_set_equipment` `native_living_refs.rs:122`; `native_carried_stack` `native_hooks.rs:8450`; `native_inventory_tick` `:7614`; API 13/39 `docs/native-mods.md:179,326` | usable | **realizable now** | Add `equip.rs` + `use_item` injection. Track the selected hotbar slot from `inventory_tick`’s `selected` flag (the hook does not name the hand), clone the used stack, `set_equipment(OffHand, …)`, then clear the source through `carried_stack(slot)`. Do **not** use `take_items` once a `minecraft:damage` component is present (`native_server.rs:503-505`). E2E: right-click equip, drop, wrong-slot refusal. |
| A3 | Durability, breakage (`notes.md:121-123`) | `native_inventory_tick` `native_hooks.rs:7614`; `native_carried_stack` `native_hooks.rs:8450`; `native_set_component` `native_components.rs:49`; `native_item_damaged` `native_hooks.rs:7476`; `native_player_destroy_item` `native_hooks.rs:7425`; API 39/65/162/315 `docs/native-mods.md:326,825,1202,1510` | usable | **realizable now** | Add a pure wear table (uses/hurt/mining per artifact) and an `inventory_tick` helper; write `minecraft:damage` with `native_set_component`, clear the slot at `max_damage`, and let `item_damaged` only reduce/cancel vanilla double-wear. E2E: 100 uses → break. |
| A4 | Cooldowns (`notes.md:124-125`) | `native_player_item_cooldown` `native_hooks.rs:4647` (cancel only); own timer via `player_tick`/`inventory_tick`; API 262 `docs/native-mods.md:1402` | usable (cancel + own timer); starting a vanilla cooldown is not offered | **realizable now** (own cooldown; the vanilla cooldown overlay is not exposed) | Keep `next_use_tick` per player in `support.rs`; refuse in `use_item`; use `native_player_item_cooldown` only to stop a *vanilla* cooldown. E2E: two clicks 1 s apart → one effect. |
| A5 | Retaliation, particles, sounds (`notes.md:126-128`) | `native_summon` `native_entities.rs:141` (`minecraft:lightning_bolt` as `/summon`); `native_play_sound` `native_hooks.rs:8193`; `native_send_particles` `native_hooks.rs:8233`; `native_flame_particle` `native_hooks.rs:8426`; API 59/25/11 `docs/native-mods.md:802,234,152` | usable | **realizable now** | On a cancelled lightning hit, queue the 25 % retaliation in `player_tick`/`server_tick`, `summon` the bolt at the attacker, play the hurt sound and particles. Add a config `retaliation_chance` so the e2e can force 0/1. |
| A6 | Fall-multiplier default (`notes.md:129-131`) | none needed — config default | usable (config) | **realizable now** (the port already uses upstream’s `0`; `-1` is the documented pack option) | Leave as is; this is a clarifying note, not a mismatch. No API work. |
| A7 | The other ~45 artifacts (`notes.md:132-133`) | content items + the existing attribute/equipment/attack/damage hooks; `native_hurt_enemy` `native_hooks.rs:7601`, `native_living_incoming_damage` `native_hooks.rs:7709`; API 12/39/50 `docs/native-mods.md:175,326,707` | usable | **realizable now** (content-heavy) | Batch by effect family: first the attribute-only artifacts (content + `player_tick`), then the on-hit family. No new hook request per item. |
| A8 | The Mimic (`notes.md:132-133`) | `native_spawn_mob` `native_mobs.rs:166`; `native_mob_submit` `native_hooks.rs:3248`; `native_entity_tick_post` `native_hooks.rs:7350`; `native_push_mob` `native_mobs.rs:258` / `native_teleport_mob` `:236`; custom entity types only copy `minecraft:snowball` (`docs/mods.md:301-317`) | usable as an approximation; **custom entity type/AI absent** | **partial** — a re-skinned vanilla mob with hand-rolled `entity_tick` AI is possible; a true `artifacts:mimic` entity is not | Defer the faithful Mimic. If a chest-like ambusher is wanted, prototype a zombie/villager base + `mob_submit` skin + tick AI and record every divergence; otherwise file “custom entity type + goal AI” as an API request. |
| A9 | Campsite loot (`notes.md:132-133`) | `horizon_world::loot::native_loot_table_load` `crates/horizon-world/src/loot.rs:589`; content structures/worldgen; API 75 `docs/native-mods.md:878` | usable | **realizable now** (content-only) | Carry the campsite structure/loot data and edit the tables in `loot_table_load`; no native code needed for the loot. |
| A10 | Config UI (`notes.md:132-133`) | `native_register_configs` `native_hooks.rs:2424`; `native_config_loading` `native_hooks.rs:2432`; `native_mod_screen_open` `native_hooks.rs:8968`; `native_screen_render_post` `native_hooks.rs:3372`; `native_mouse_button_pre` `native_hooks.rs:2580`; API 397/111 `docs/native-mods.md:1678,111` | usable | **realizable now** | Keep the flat file for now; add a mod screen with toggles that write the same keys (or migrate to a registered `Server` config). E2E: open, toggle, close, behaviour changes. |
| A11 | Client tooltip (`notes.md:134-135`) | `native_tooltip` `native_hooks.rs:6413` (`collect: lines`); `native_tooltip_color` `native_hooks.rs:6418`; API 12/368 `docs/native-mods.md:175,368` | usable | **realizable now** | Append attribute deltas and the ability line per artifact; a pure `tooltip_lines(stack)` in `runtime.rs`, a thin client helper. Probe with `ctl` hover or a screenshot e2e. |
| A12 | One-tick loadout cache (`notes.md:136-139`) | `native_living_incoming_damage` `native_hooks.rs:7709` is still **static**; `native_living_incoming_damage_v2` `native_hooks.rs:7736` is player→mob only (`docs/native-mods.md:1652`); `NativeServerPlayer.entity_id` `mod_api.rs:385` only maps ids | **absent** for the needed host-aware player-victim hook | **blocked** — a pickup and a lightning hit in the same tick still miss the pendant; no helper can read the inventory in v1 | Keep the one-tick cache. File an API request for a host-aware damage hook that also fires for player victims (a v3, not `v2`); do not schedule port work until it lands. |

## Summary

- **Deferred features audited: 12** (the nine `notes.md` gap bullets, with
  “other artifacts / Mimic / campsite / config UI” split into A7–A10).
- **Realizable now in full: 9** — A2–A7, A9–A11 (A6 is a config-only
  deviation; A7 is content-heavy but has no API blocker).
- **Newly opened or newly practical at `83ad384f`: 6** — A1 (vanilla
  equipment read/write), A3 (per-stack tick + component writes), A4 (cooldown
  cancel), A5 (summon/particles/sound helpers), A10 (registered config + mod
  screen), A11 (tooltip collection). The rest were deferred scope rather than
  hard API gaps.
- **Partial: 2** — A1 (no custom artifact slot) and A8 (no custom mob
  type/AI; re-skin approximation only).
- **Still blocked: 1** — A12 (the static damage hook's one-tick blind spot).
- **Highest-value next step:** implement **A2 right-click equip onto the off
  hand**, with A3’s durability write in the same helper. It removes the most
  visible slice-1 deviation (“the items are inert on use”), exercises the new
  equipment/component helpers, and is small enough for one fail-capable e2e
  (equip → attribute appears; drop → attribute gone; broken item → gone).
  A11 (tooltip) is the cheapest follow-up and makes the new equipment
  behaviour legible in-game.

## Gotchas found while reading

- `living_set_equipment` explicitly returns false for a player’s body/saddle
  slot (`native_living_refs.rs:113-119`); the port must target
  `OffHand`/armour, not a “curio” slot.
- `take_items`/`count_items` only see “plain” stacks — `damage == 0` and no
  extra components (`native_server.rs:503-505`). Once A3 writes a damage
  component, equip/unequip must use `native_carried_stack` or the inventory
  helpers, not the by-id item helpers.
- `native_inventory_tick`’s slot numbers are the inventory’s saved numbers
  (`0..35`, armour `100..103`, off hand `-106`, `native_hooks.rs:7604-7609`),
  so the same helper can cover worn and carried artifacts.
- `native_use_item` does not name the hand. Track the selected slot from
  `inventory_tick`’s `selected` flag (or `player_item_held`) and compare with
  `native_held()`; if the artifact is already in the off hand, setting the off
  hand again and then clearing “the source” would delete it.
- The new v1 damage hook can change `amount` as well as `canceled`. The
  existing `damage.rs` binding stays valid as-is (it does not bind `amount`);
  a future scaling artifact adds an `amount` binding and needs no second hook.
