// Game-side glue for Artifacts slice 1. Injected into `crate::game` as
// `artifacts_state`; it needs the game, so it is not part of the standalone
// library (only `runtime.rs` is).
//
// Upstream equips artifacts in its own slots (Curios on 1.21.1). This slice
// defers the slot system, so the player's carried main inventory and off hand
// stand in for the worn slots. Every player tick the glue scans that inventory,
// applies the three artifacts' attribute modifiers, and caches which artifacts
// the player carries for the static `living_incoming_damage` hook, which cannot
// read the game itself.

use std::collections::BTreeMap;
use std::fs;
use std::sync::Mutex;

use horizon_client::items::ItemStack;

use crate::game::artifacts::{
    CONFIG_FILE, Carried, Config, attribute_effects, blocks_damage, equips_from_use,
    format_config, parse_config,
};

#[derive(Default)]
struct Core {
    config: Config,
    /// Which artifacts each player carries, keyed by entity id (the id the
    /// damage hook names). Refreshed every `player_tick`.
    carried: BTreeMap<i32, Carried>,
}

static CORE: Mutex<Option<Core>> = Mutex::new(None);

fn core() -> std::sync::MutexGuard<'static, Option<Core>> {
    CORE.lock().unwrap_or_else(|poison| poison.into_inner())
}

/// The loaded server settings, or upstream's defaults before the server
/// reaches `server_about_to_start`.
fn config() -> Config {
    let guard = core();
    guard
        .as_ref()
        .map_or_else(Config::default, |core| core.config.clone())
}

/// Before the server starts: read `config/artifacts.conf`, writing the
/// defaults when it is absent.
#[allow(private_interfaces)]
pub fn on_server_about_to_start(_game: &mut crate::game::Game<'_>) {
    let mut config = Config::default();
    match crate::game::Game::native_config_dir() {
        Ok(dir) => {
            let path = dir.join(CONFIG_FILE);
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(_) => {
                    let text = format_config(&config);
                    if fs::create_dir_all(&dir)
                        .and_then(|()| fs::write(&path, &text))
                        .is_err()
                    {
                        log::warn!("[artifacts] could not write {}", path.display());
                    }
                    text
                }
            };
            config = parse_config(&text);
            log::info!(
                "[artifacts] config {} (bunny_hoppers={}, feral_claws={}, shock_pendant={})",
                path.display(),
                config.bunny_hoppers_enabled,
                config.feral_claws_enabled,
                config.shock_pendant_enabled
            );
        }
        Err(error) => log::warn!("[artifacts] no config dir: {error}"),
    }
    let mut guard = core();
    let core = guard.get_or_insert_with(Core::default);
    core.config = config;
}

/// End of a player's server tick: count the artifacts they carry, apply the
/// attribute modifiers and remember the loadout for the damage hook.
#[allow(private_interfaces)]
pub fn on_player_tick(game: &mut crate::game::Game<'_>) {
    let Some(player) = game.native_acting_player() else {
        return;
    };
    let uuid = player.uuid;
    let carried = scan_carried(game, uuid);
    let config = config();
    apply_attributes(game, &carried, &config);
    let entity_ids = entity_ids_of(game, Some(uuid));
    let mut guard = core();
    let core = guard.get_or_insert_with(Core::default);
    for entity_id in entity_ids {
        core.carried.insert(entity_id, carried);
    }
}

/// A right click with an artifact (`use_item`, API 13): move the held stack to
/// the off hand, the port's worn slot for artifacts. Upstream equips into its
/// own slots (`EquipmentSlotManager.java:30-55`); with no custom slot registry
/// at this pin, the off hand is the one real worn slot. The inventory scan
/// already counts the off hand, so the attributes stay applied. The pure
/// decision (`equips_from_use`) refuses a non-artifact and an occupied off
/// hand, so an equip cannot silently replace an item.
#[allow(private_interfaces)]
pub fn on_use_item(game: &mut crate::game::Game<'_>, stack: &ItemStack, claimed: &mut bool) {
    if !equips_from_use(&stack.item, game.offhand.is_some()) {
        return;
    }
    // Only the main hand's own use may equip; an off-hand use must not move
    // the main-hand stack.
    if !game.held().is_some_and(|held| held.item == stack.item) {
        return;
    }
    let Some(held) = game.held_mut().take() else {
        return;
    };
    game.offhand = Some(held);
    *claimed = true;
}

/// A hit is about to land (`living_incoming_damage`, API 50). The hook is
/// static, so the loadout comes from the cache `on_player_tick` refreshed.
/// Upstream makes the wearer invulnerable to `#minecraft:is_lightning`
/// (`LivingEntityMixin.java:14-20`); the port cancels the same hit.
pub fn on_living_incoming_damage(entity: i32, kind: &str, damage_type: &str, canceled: &mut bool) {
    if *canceled {
        return;
    }
    let config = config();
    if !config.shock_pendant_enabled || !config.shock_pendant_cancel_lightning_damage {
        return;
    }
    let carried = {
        let guard = core();
        guard
            .as_ref()
            .and_then(|core| core.carried.get(&entity).copied())
            .unwrap_or_default()
    };
    if blocks_damage(&carried, kind, damage_type, &config) {
        *canceled = true;
    }
}

/// The artifacts in one player's main inventory and off hand. A LAN guest's
/// inventory lives in the hub's guest record, not in the host's slots.
fn scan_carried(game: &crate::game::Game<'_>, uuid: u128) -> Carried {
    if let Some(host) = game.lan.as_ref()
        && let Some(guest) = host
            .hub
            .guests
            .iter()
            .find(|guest| guest.placed && !guest.dead && guest.uuid == uuid)
    {
        let stacks = guest
            .inventory
            .iter()
            .flatten()
            .map(|stack| (stack.item.as_ref(), stack.count));
        let mut carried = Carried::from_stacks(stacks);
        if let Some(stack) = guest.offhand.as_ref() {
            carried.add_stack(stack.item.as_ref(), stack.count);
        }
        return carried;
    }
    let stacks = game
        .inventory
        .iter()
        .flatten()
        .map(|stack| (stack.item.as_ref(), stack.count));
    let mut carried = Carried::from_stacks(stacks);
    if let Some(stack) = game.offhand.as_ref() {
        carried.add_stack(stack.item.as_ref(), stack.count);
    }
    carried
}

/// The entity ids a player can be hurt by. A guest's body takes its hub id; a
/// local owner is the hub's `host_entity`, but an integrated server's
/// `PlayerMotion.entity_id` can still be 0 (it is only filled in when no
/// server thread runs), so the owner's loadout is cached under both ids.
fn entity_ids_of(game: &crate::game::Game<'_>, uuid: Option<u128>) -> Vec<i32> {
    // At this pin a LAN host no longer carries a separate `host_entity`; the
    // local body's entity id is the host's, and a guest's record names theirs.
    let local = game.player.as_ref().map_or(0, |player| player.entity_id);
    let Some(uuid) = uuid else {
        return vec![local];
    };
    let Some(host) = game.lan.as_ref() else {
        return vec![local];
    };
    if let Some(guest) = host
        .hub
        .guests
        .iter()
        .find(|guest| guest.placed && !guest.dead && guest.uuid == uuid)
    {
        return vec![guest.entity_id];
    }
    vec![local]
}

/// Set or remove each artifact's attribute modifier. The pin's modifiers are
/// transient, so this runs every player tick; a dropped or disabled artifact
/// gets `present = false` and its modifier is removed.
fn apply_attributes(game: &mut crate::game::Game<'_>, carried: &Carried, config: &Config) {
    let effects = attribute_effects(carried, config);
    let Some(player) = game.player.as_mut() else {
        return;
    };
    for effect in effects {
        if let Err(error) = player.set_modifier(
            effect.attribute,
            effect.id,
            effect.amount,
            effect.operation.as_str(),
            effect.present,
        ) {
            log::warn!("[artifacts] modifier {}: {error}", effect.id);
        }
    }
}
