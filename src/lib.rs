//! Artifacts for Horizon — the SDK package: code and carried content in one `.hmod`.
//!
//! Ported from Artifacts 15.1.3 (ochotonida/artifacts, MIT); see `notes.md` and
//! `FINDINGS.md` for the behaviour and its evidence. The pure rules live in [`runtime`]
//! and are unit-tested by `cargo test`; this file is the glue — the one `impl Hooks`,
//! the artifact equipment [`ItemPart`] and the item keys the rules are translated from.
//!
//! Upstream equips artifacts in its own slots (Curios on 1.21.1). The port keeps 0.1.0's
//! standing in for them: the main inventory and the off hand, so carrying an artifact is
//! the same as wearing it. Four moments:
//!
//! - `server_about_to_start`: read `config/artifacts.conf`, writing the defaults when it
//!   is absent.
//! - `player_tick_post`: scan the server player's inventory, apply the three artifacts'
//!   attribute modifiers and cache the loadout the damage hook reads.
//! - the three artifact items' `use_item`: a right click moves the stack into an empty
//!   off hand, the port's one worn slot.
//! - `living_incoming_damage`: the Shock Pendant cancels a lightning hit.
//!
//! The content (`assets/`, `data/`) is declared by this same package, so a requested item
//! key resolves against the mod's own items; the retired bundle's embedded-content
//! injection has no job left.
pub mod runtime;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::sync::OnceLock;

use mod_api::{
    self, AttributeModifier, AttributeOperation, ConfigFileError, ConfigFiles, ConfigPath,
    EquipmentSlot, Flow, Hand, Hooks, HostSide, Identifier, InteractionResult, ItemKey, ItemPart,
    LevelRandomHost, LivingIncomingDamageEvent, ModifierLifetime, PlayerLoggedOutEvent, PlayerRef,
    PlayerTickPostEvent, Registrar, RegistrationContext, ServerAboutToStartEvent, ServerHost,
    UseItemEvent,
};

/// The main inventory's highest `Inventory` slot number: 0 to 35 is the main inventory
/// (0 to 8 the hotbar).
const MAIN_INVENTORY_LAST: u32 = 35;

/// The off hand's `Inventory` slot number (`Inventory.EQUIPMENT_SLOT_MAPPING`), the one
/// worn slot the API gives an artifact here.
const OFF_HAND_SLOT: u32 = 40;

/// The keys the runtime's ids translate to, resolved once at registration.
///
/// The runtime names items by their registry id (`artifacts:feral_claws`), which is what
/// makes it testable without a game; the API identifies a stack's item by [`ItemKey`],
/// which is what a stack answers. This is the translation table, and it is why the scan
/// below only converts the three ids the rules know.
struct Ids {
    bunny_hoppers: ItemKey,
    feral_claws: ItemKey,
    shock_pendant: ItemKey,
}

static IDS: OnceLock<Ids> = OnceLock::new();

impl Ids {
    /// The artifact `key` names, or `None` for any other item.
    fn artifact(&self, key: ItemKey) -> Option<runtime::Artifact> {
        if key == self.bunny_hoppers {
            Some(runtime::Artifact::BunnyHoppers)
        } else if key == self.feral_claws {
            Some(runtime::Artifact::FeralClaws)
        } else if key == self.shock_pendant {
            Some(runtime::Artifact::ShockPendant)
        } else {
            None
        }
    }
}

/// The server settings in force and the loadout cache the static damage hook reads.
#[derive(Default)]
struct State {
    config: runtime::Config,
    /// Which artifacts each player carries, keyed by the entity id the damage hook names.
    /// Refreshed at the end of every server player tick.
    carried: BTreeMap<i32, runtime::Carried>,
}

/// The one mod instance: the macro boxes one `Default` for the session, and `Hooks` is
/// `Send + Sync`, so the state is behind a `Mutex`.
#[derive(Default)]
struct Mod {
    state: Mutex<State>,
}

impl Mod {
    /// The state, for one short read or write that calls no host method.
    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .expect("the artifacts state lock is held for one short read or write")
    }
}

/// The artifact's own behaviour: attached to each of the three items as a Part, so the
/// game calls it only for a stack of one of them (the retired bundle's global `use_item`
/// helper had to compare the stack itself).
struct ArtifactPart;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Resolve the three item keys and attach the equipment Part to each item. The items
    /// are declared by this same package, and a requested key is validated after all
    /// declarations, so a missing item refuses the mod instead of failing silently.
    fn register(&self, r: &mut dyn Registrar, _context: &RegistrationContext) {
        let ids = Ids {
            bunny_hoppers: r.item_key(runtime::Artifact::BunnyHoppers.item_id()),
            feral_claws: r.item_key(runtime::Artifact::FeralClaws.item_id()),
            shock_pendant: r.item_key(runtime::Artifact::ShockPendant.item_id()),
        };
        r.item_part(ids.bunny_hoppers, ArtifactPart);
        r.item_part(ids.feral_claws, ArtifactPart);
        r.item_part(ids.shock_pendant, ArtifactPart);
        let _ = IDS.set(ids);
    }

    /// The server is about to open its worlds: read `config/artifacts.conf`, writing the
    /// upstream defaults when the file is absent.
    fn server_about_to_start(
        &self,
        host: &mut dyn ServerHost,
        _event: &mut ServerAboutToStartEvent,
    ) {
        let config = load_config(host.config_files());
        self.state().config = config;
    }

    /// The end of a player's server tick: rebuild the loadout from the inventory they
    /// carry, set or remove each artifact's attribute modifiers, and remember the loadout
    /// for `living_incoming_damage`, which is handed the level only and cannot read an
    /// inventory.
    fn player_tick_post(&self, host: HostSide, event: &mut PlayerTickPostEvent) {
        // The hook runs on both logical sides; only the server owns the inventory and may
        // write an attribute, so a client-side tick has nothing to do.
        let HostSide::Server(host) = host else {
            return;
        };
        let Some(ids) = IDS.get() else {
            return;
        };
        let player = event.player;
        let carried = scan(host, ids, player);
        let config = self.state().config.clone();
        apply_attributes(host, player, &carried, &config);
        // The damage hook names the entity as an `i32`; a handle's id is the same number.
        if let Ok(key) = i32::try_from(player.entity().id()) {
            self.state().carried.insert(key, carried);
        }
    }

    /// A hit is about to land on `event.entity`: cancel a lightning hit on a player whose
    /// loadout has the Shock Pendant. Upstream makes the wearer invulnerable to
    /// `#minecraft:is_lightning` (`LivingEntityMixin.java:14-20`), which is what a cancel
    /// does here — no damage, hurt sound, knockback, cooldown or statistics.
    fn living_incoming_damage(
        &self,
        _level: &mut dyn LevelRandomHost,
        event: &mut LivingIncomingDamageEvent<'_>,
    ) -> Flow {
        if event.cancelled {
            return Flow::Continue;
        }
        let carried = self
            .state()
            .carried
            .get(&event.entity)
            .copied()
            .unwrap_or_default();
        let config = self.state().config.clone();
        if runtime::blocks_damage(&carried, event.kind, event.source.damage_type, &config) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }

    /// A player left: drop the whole loadout cache. A later player can be given the entity
    /// id of one who left, and the next tick of everyone still online refills it.
    fn player_logged_out(&self, _host: &mut dyn ServerHost, _event: &mut PlayerLoggedOutEvent<'_>) {
        self.state().carried.clear();
    }
}

#[mod_api::horizon_part]
impl ItemPart for ArtifactPart {
    /// A player right-clicked this artifact in the main hand: move it into an empty off
    /// hand, the port's one worn slot (`tryEquipFromUse`, audit A2). An occupied off hand
    /// is never replaced and the off hand's own use never moves the main-hand stack, so
    /// nothing is silently overwritten.
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        if event.hand != Hand::Main {
            return None;
        }
        let ids = IDS.get()?;
        let artifact = ids.artifact(event.stack.item())?;
        let player = host.player(event.player)?;
        let living = host.living(player.entity())?;
        let off_hand_occupied = living.equipment(&*host, EquipmentSlot::OffHand).is_some();
        if !runtime::equips_from_use(artifact.item_id(), off_hand_occupied) {
            return None;
        }
        // Clear the main hand first: if that write is refused nothing has moved yet, and
        // the off hand was empty a moment ago, so the second write cannot replace a stack.
        if living
            .set_equipment(host, EquipmentSlot::MainHand, None)
            .is_err()
        {
            return None;
        }
        if living
            .set_equipment(host, EquipmentSlot::OffHand, Some(event.stack.clone()))
            .is_err()
        {
            return None;
        }
        Some(InteractionResult::SuccessServer)
    }
}

/// Read `config/artifacts.conf` through the mod's own checked config service, writing the
/// upstream defaults when the file is absent. A refused read keeps the defaults, which is
/// what upstream starts with.
fn load_config(files: &dyn ConfigFiles) -> runtime::Config {
    let Ok(path) = ConfigPath::new(runtime::CONFIG_FILE) else {
        return runtime::Config::default();
    };
    match files.read(&path) {
        Ok(bytes) => runtime::parse_config(&String::from_utf8_lossy(&bytes)),
        Err(ConfigFileError::NotFound) => {
            let config = runtime::Config::default();
            let _ = files.write(&path, runtime::format_config(&config).as_bytes());
            config
        }
        Err(_) => runtime::Config::default(),
    }
}

/// The artifacts one server player carries: the main inventory and the off hand, as 0.1.0
/// scanned them (the armour slots are not worn artifact slots upstream either).
fn scan(host: &mut dyn ServerHost, ids: &Ids, player: PlayerRef) -> runtime::Carried {
    let mut carried = runtime::Carried::new();
    let Some(slots) = player.inventory_slots(host) else {
        return carried;
    };
    for (slot, stack) in slots {
        if slot > MAIN_INVENTORY_LAST && slot != OFF_HAND_SLOT {
            continue;
        }
        if let Some(artifact) = ids.artifact(stack.item()) {
            let _ = carried.add_stack(artifact.item_id(), stack.count());
        }
    }
    carried
}

/// Set or remove each artifact's attribute modifier. The pin's modifiers are transient and
/// re-applied every tick, so a dropped or disabled artifact loses its modifier on the next
/// one.
fn apply_attributes(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    carried: &runtime::Carried,
    config: &runtime::Config,
) {
    let effects = runtime::attribute_effects(carried, config);
    let Some(living) = host.living(player.entity()) else {
        return;
    };
    for effect in effects {
        // The runtime's ids are the mod's own constants, so a refusal here is a bug in
        // them, not a runtime condition: skip that one modifier and keep the others.
        let Ok(id) = Identifier::parse(effect.id) else {
            continue;
        };
        if effect.present {
            let modifier = AttributeModifier::new(id, effect.amount, operation(effect.operation));
            let _ = living.replace_attribute_modifier(
                host,
                effect.attribute,
                modifier,
                ModifierLifetime::Transient,
            );
        } else {
            let _ = living.remove_attribute_modifier(host, effect.attribute, &id);
        }
    }
}

/// The pin's modifier operation for the runtime's.
fn operation(operation: runtime::Operation) -> AttributeOperation {
    match operation {
        runtime::Operation::AddValue => AttributeOperation::AddValue,
        runtime::Operation::AddMultipliedBase => AttributeOperation::AddMultipliedBase,
    }
}
