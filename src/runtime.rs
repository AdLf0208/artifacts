// Artifacts for Horizon — the pure effect core.
//
// Ported from Artifacts (ochotonida/artifacts, MIT), upstream read at the
// 1.21.1 branch (13.2.5) and checked against the 26.1/26.3 branches. This is a
// clean-room Rust rewrite of the behaviour, not a translation of the Java.
//
// Slice 1 carries three artifacts whose effects fit the pin:
//
// - `artifacts:bunny_hoppers` — jump strength +40 %, fall damage multiplier
//   unchanged (upstream default 0), safe fall distance +10 blocks.
// - `artifacts:feral_claws` — attack speed +30 % (ADD_MULTIPLIED_BASE).
// - `artifacts:shock_pendant` — lightning damage is cancelled while worn.
//
// Upstream equips artifacts in its own slots (Curios on 1.21.1) and only a
// worn artifact adds its modifiers. This port has one worn slot the pin
// exposes — the off hand — and a carried artifact grants nothing: the effect
// source is the worn slot, not the whole inventory. Each artifact counts at
// most once, like one equipment slot.

/// Where the glue keeps the server settings (upstream uses a NeoForge config).
pub const CONFIG_FILE: &str = "artifacts.conf";

/// The one worn slot this port has: `Inventory`'s off-hand number, as
/// `player_inventory_slots` numbers slots (0 to 35 main, 36 to 39 armour,
/// 40 off hand, 41 body, 42 saddle).
pub const WORN_SLOT: u32 = 40;

/// Whether an `Inventory` slot number is the port's worn slot. Only the off
/// hand is worn; everything else is merely carried.
#[must_use]
pub fn is_worn_slot(slot: u32) -> bool {
    slot == WORN_SLOT
}

/// Upstream's config defaults (`ItemConfigs.java:152-173,491-501,880-903`).
pub const BUNNY_HOPPERS_FALL_DAMAGE_MULTIPLIER_DEFAULT: f64 = 0.0;
pub const BUNNY_HOPPERS_SAFE_FALL_DISTANCE_BONUS_DEFAULT: f64 = 10.0;
pub const BUNNY_HOPPERS_JUMP_STRENGTH_BONUS_DEFAULT: f64 = 0.40;
pub const FERAL_CLAWS_ATTACK_SPEED_BONUS_DEFAULT: f64 = 0.30;

/// The port's accepted ranges for the four numeric settings. Upstream's
/// `ValueTypes.ATTRIBUTE_MODIFIER` accepts large values; these caps only keep
/// a typo from producing an absurd attribute. `-1` is valid for the
/// `ADD_MULTIPLIED_BASE` bonuses (it cancels the base value).
pub const FALL_DAMAGE_MULTIPLIER_MIN: f64 = -10.0;
pub const FALL_DAMAGE_MULTIPLIER_MAX: f64 = 10.0;
pub const SAFE_FALL_DISTANCE_MIN: f64 = -1024.0;
pub const SAFE_FALL_DISTANCE_MAX: f64 = 1024.0;
pub const JUMP_STRENGTH_MIN: f64 = -1.0;
pub const JUMP_STRENGTH_MAX: f64 = 32.0;
pub const ATTACK_SPEED_MIN: f64 = -1.0;
pub const ATTACK_SPEED_MAX: f64 = 32.0;

/// The damage type tag upstream's Shock Pendant protects against
/// (`ModItems.java:286-292`). 26.3's `#minecraft:is_lightning` holds exactly
/// this one entry.
pub const LIGHTNING_DAMAGE_TYPE: &str = "minecraft:lightning_bolt";

/// The three slice-1 artifacts (`ModItems.java:462-467,572-587`, plus
/// `:275-293`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Artifact {
    BunnyHoppers,
    FeralClaws,
    ShockPendant,
}

impl Artifact {
    /// Every slice-1 artifact.
    pub const ALL: [Artifact; 3] = [
        Artifact::BunnyHoppers,
        Artifact::FeralClaws,
        Artifact::ShockPendant,
    ];

    /// The registry id of this artifact.
    #[must_use]
    pub fn item_id(self) -> &'static str {
        match self {
            Artifact::BunnyHoppers => "artifacts:bunny_hoppers",
            Artifact::FeralClaws => "artifacts:feral_claws",
            Artifact::ShockPendant => "artifacts:shock_pendant",
        }
    }

    /// The artifact a registry id names, or None for any other item.
    #[must_use]
    pub fn from_item(item: &str) -> Option<Artifact> {
        Artifact::ALL
            .into_iter()
            .find(|artifact| artifact.item_id() == item)
    }

    /// The artifact's index in [`Artifact::ALL`].
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Artifact::BunnyHoppers => 0,
            Artifact::FeralClaws => 1,
            Artifact::ShockPendant => 2,
        }
    }
}

/// The artifacts a player wears. Upstream has one slot per artifact kind, so a
/// second copy adds nothing; the port records presence, not count, and only the
/// worn slot contributes (see [`worn_from_slots`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loadout {
    present: [bool; 3],
}

impl Loadout {
    /// No artifact worn.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record one worn stack of `artifact`. A zero-count stack wears nothing.
    fn wear(&mut self, artifact: Artifact, count: u32) {
        if count > 0 {
            self.present[artifact.index()] = true;
        }
    }

    /// Whether this artifact is worn.
    #[must_use]
    pub fn has(self, artifact: Artifact) -> bool {
        self.present[artifact.index()]
    }
}

/// Build the worn loadout from `(Inventory slot, artifact, stack count)` rows:
/// only the off hand counts, so the same artifact in the main inventory or in
/// an armour slot grants nothing.
#[must_use]
pub fn worn_from_slots(slots: impl IntoIterator<Item = (u32, Artifact, u32)>) -> Loadout {
    let mut worn = Loadout::new();
    for (slot, artifact, count) in slots {
        if is_worn_slot(slot) {
            worn.wear(artifact, count);
        }
    }
    worn
}

/// A vanilla attribute modifier operation, named as the pin's
/// `PlayerMotion::set_modifier` takes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    AddValue,
    AddMultipliedBase,
}

impl Operation {
    /// The operation's pin name.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Operation::AddValue => "add_value",
            Operation::AddMultipliedBase => "add_multiplied_base",
        }
    }
}

/// One attribute modifier the glue should set or remove.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttributeEffect {
    /// The vanilla attribute id (`minecraft:attack_speed`).
    pub attribute: &'static str,
    /// The modifier id, upstream's `<mod>/<item>/<attribute path>` shape
    /// (`ArtifactProperties.java:106-111`).
    pub id: &'static str,
    /// The modifier amount; ignored when `present` is false.
    pub amount: f64,
    pub operation: Operation,
    /// Whether the artifact is worn and enabled. False means the glue must
    /// remove the modifier (it may be left from an earlier tick).
    pub present: bool,
}

/// Every attribute modifier the three artifacts can apply, in a fixed order.
/// The glue calls `set_modifier` for each one with `present`, so an artifact
/// taken off or disabled removes its modifier on the next tick.
#[must_use]
pub fn attribute_effects(worn: &Loadout, config: &ServerConfig) -> Vec<AttributeEffect> {
    vec![
        AttributeEffect {
            attribute: "minecraft:attack_speed",
            id: "artifacts:feral_claws/attack_speed",
            amount: config.feral_claws_attack_speed_bonus,
            operation: Operation::AddMultipliedBase,
            present: config.feral_claws_enabled && worn.has(Artifact::FeralClaws),
        },
        AttributeEffect {
            attribute: "minecraft:jump_strength",
            id: "artifacts:bunny_hoppers/jump_strength",
            amount: config.bunny_hoppers_jump_strength_bonus,
            operation: Operation::AddMultipliedBase,
            present: config.bunny_hoppers_enabled && worn.has(Artifact::BunnyHoppers),
        },
        AttributeEffect {
            attribute: "minecraft:fall_damage_multiplier",
            id: "artifacts:bunny_hoppers/fall_damage_multiplier",
            amount: config.bunny_hoppers_fall_damage_multiplier,
            operation: Operation::AddMultipliedBase,
            present: config.bunny_hoppers_enabled && worn.has(Artifact::BunnyHoppers),
        },
        AttributeEffect {
            attribute: "minecraft:safe_fall_distance",
            id: "artifacts:bunny_hoppers/safe_fall_distance",
            amount: config.bunny_hoppers_safe_fall_distance_bonus,
            operation: Operation::AddValue,
            present: config.bunny_hoppers_enabled && worn.has(Artifact::BunnyHoppers),
        },
    ]
}

/// Whether a damage type is in `#minecraft:is_lightning`
/// (`DamageImmunity.java:17-41`).
#[must_use]
pub fn is_lightning(damage_type: &str) -> bool {
    damage_type == LIGHTNING_DAMAGE_TYPE
}

/// Whether the Shock Pendant cancels this hit. Upstream makes the wearer
/// invulnerable to `#minecraft:is_lightning` (`LivingEntityMixin.java:14-20`);
/// the port answers the same question at `living_incoming_damage`.
#[must_use]
pub fn blocks_damage(worn: &Loadout, kind: &str, damage_type: &str, config: &ServerConfig) -> bool {
    kind == "minecraft:player"
        && config.shock_pendant_enabled
        && config.shock_pendant_cancel_lightning_damage
        && worn.has(Artifact::ShockPendant)
        && is_lightning(damage_type)
}

/// Whether a right click on `item` moves it to the off hand: only an artifact,
/// and only when the off hand is empty. Upstream's `tryEquipFromUse` equips
/// into its own slot; the port's one worn slot is the off hand, so an occupied
/// off hand is never replaced and a non-artifact is never moved.
#[must_use]
pub fn equips_from_use(item: &str, off_hand_occupied: bool) -> bool {
    !off_hand_occupied && Artifact::from_item(item).is_some()
}

/// The server settings this slice understands. Defaults are upstream's.
#[derive(Clone, Debug, PartialEq)]
pub struct ServerConfig {
    pub bunny_hoppers_enabled: bool,
    pub bunny_hoppers_fall_damage_multiplier: f64,
    pub bunny_hoppers_safe_fall_distance_bonus: f64,
    pub bunny_hoppers_jump_strength_bonus: f64,
    pub feral_claws_enabled: bool,
    pub feral_claws_attack_speed_bonus: f64,
    pub shock_pendant_enabled: bool,
    pub shock_pendant_cancel_lightning_damage: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bunny_hoppers_enabled: true,
            bunny_hoppers_fall_damage_multiplier: BUNNY_HOPPERS_FALL_DAMAGE_MULTIPLIER_DEFAULT,
            bunny_hoppers_safe_fall_distance_bonus: BUNNY_HOPPERS_SAFE_FALL_DISTANCE_BONUS_DEFAULT,
            bunny_hoppers_jump_strength_bonus: BUNNY_HOPPERS_JUMP_STRENGTH_BONUS_DEFAULT,
            feral_claws_enabled: true,
            feral_claws_attack_speed_bonus: FERAL_CLAWS_ATTACK_SPEED_BONUS_DEFAULT,
            shock_pendant_enabled: true,
            shock_pendant_cancel_lightning_damage: true,
        }
    }
}

/// Parse `config/artifacts.conf`: `key = value` lines, `#` comments. Unknown
/// keys and badly typed values keep their defaults; numbers are clamped and a
/// non-finite value keeps the default.
#[must_use]
pub fn parse_config(text: &str) -> ServerConfig {
    let mut config = ServerConfig::default();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "bunny_hoppers_enabled" => {
                config.bunny_hoppers_enabled =
                    value.parse().unwrap_or(config.bunny_hoppers_enabled);
            }
            "bunny_hoppers_fall_damage_multiplier" => {
                config.bunny_hoppers_fall_damage_multiplier = number(
                    value,
                    config.bunny_hoppers_fall_damage_multiplier,
                    FALL_DAMAGE_MULTIPLIER_MIN,
                    FALL_DAMAGE_MULTIPLIER_MAX,
                );
            }
            "bunny_hoppers_safe_fall_distance_bonus" => {
                config.bunny_hoppers_safe_fall_distance_bonus = number(
                    value,
                    config.bunny_hoppers_safe_fall_distance_bonus,
                    SAFE_FALL_DISTANCE_MIN,
                    SAFE_FALL_DISTANCE_MAX,
                );
            }
            "bunny_hoppers_jump_strength_bonus" => {
                config.bunny_hoppers_jump_strength_bonus = number(
                    value,
                    config.bunny_hoppers_jump_strength_bonus,
                    JUMP_STRENGTH_MIN,
                    JUMP_STRENGTH_MAX,
                );
            }
            "feral_claws_enabled" => {
                config.feral_claws_enabled = value.parse().unwrap_or(config.feral_claws_enabled);
            }
            "feral_claws_attack_speed_bonus" => {
                config.feral_claws_attack_speed_bonus = number(
                    value,
                    config.feral_claws_attack_speed_bonus,
                    ATTACK_SPEED_MIN,
                    ATTACK_SPEED_MAX,
                );
            }
            "shock_pendant_enabled" => {
                config.shock_pendant_enabled =
                    value.parse().unwrap_or(config.shock_pendant_enabled);
            }
            "shock_pendant_cancel_lightning_damage" => {
                config.shock_pendant_cancel_lightning_damage = value
                    .parse()
                    .unwrap_or(config.shock_pendant_cancel_lightning_damage);
            }
            _ => {}
        }
    }
    config
}

/// A finite `value` clamped to `min..=max`, or `default` when it is missing or
/// not a finite number.
fn number(value: &str, default: f64, min: f64, max: f64) -> f64 {
    value
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
        .map_or(default, |value| value.clamp(min, max))
}

/// Write the config in the format `parse_config` reads.
#[must_use]
pub fn format_config(config: &ServerConfig) -> String {
    format!(
        "# Artifacts (Horizon) — server settings.\n\
         # Upstream's defaults; -1 cancels an ADD_MULTIPLIED_BASE bonus.\n\
         bunny_hoppers_enabled = {}\n\
         bunny_hoppers_fall_damage_multiplier = {}\n\
         bunny_hoppers_safe_fall_distance_bonus = {}\n\
         bunny_hoppers_jump_strength_bonus = {}\n\
         feral_claws_enabled = {}\n\
         feral_claws_attack_speed_bonus = {}\n\
         shock_pendant_enabled = {}\n\
         shock_pendant_cancel_lightning_damage = {}\n",
        config.bunny_hoppers_enabled,
        config.bunny_hoppers_fall_damage_multiplier,
        config.bunny_hoppers_safe_fall_distance_bonus,
        config.bunny_hoppers_jump_strength_bonus,
        config.feral_claws_enabled,
        config.feral_claws_attack_speed_bonus,
        config.shock_pendant_enabled,
        config.shock_pendant_cancel_lightning_damage,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ServerConfig {
        ServerConfig::default()
    }

    fn effect<'a>(effects: &'a [AttributeEffect], attribute: &str) -> &'a AttributeEffect {
        effects
            .iter()
            .find(|effect| effect.attribute == attribute)
            .unwrap_or_else(|| panic!("no effect for {attribute}"))
    }

    /// A loadout with these artifacts worn in the off hand.
    fn worn(artifacts: impl IntoIterator<Item = Artifact>) -> Loadout {
        worn_from_slots(
            artifacts
                .into_iter()
                .map(|artifact| (WORN_SLOT, artifact, 1)),
        )
    }

    #[test]
    fn artifact_ids_match_upstream() {
        assert_eq!(Artifact::BunnyHoppers.item_id(), "artifacts:bunny_hoppers");
        assert_eq!(Artifact::FeralClaws.item_id(), "artifacts:feral_claws");
        assert_eq!(Artifact::ShockPendant.item_id(), "artifacts:shock_pendant");
        for artifact in Artifact::ALL {
            assert_eq!(Artifact::from_item(artifact.item_id()), Some(artifact));
        }
    }

    #[test]
    fn an_artifact_equips_only_into_an_empty_off_hand() {
        assert!(
            equips_from_use("artifacts:feral_claws", false),
            "an artifact moves into an empty off hand"
        );
        assert!(
            !equips_from_use("artifacts:feral_claws", true),
            "an occupied off hand is never replaced"
        );
        assert!(
            !equips_from_use("minecraft:stick", false),
            "a non-artifact is never moved"
        );
        assert!(
            !equips_from_use("minecraft:stick", true),
            "a non-artifact in an occupied off hand is never moved"
        );
    }

    #[test]
    fn non_artifacts_are_ignored() {
        for other in [
            "minecraft:stick",
            "artifacts:bunny_hoppers_broken",
            "artifacts:running_shoes",
            "artifacts:bunny_hoppers2",
            "",
        ] {
            assert_eq!(Artifact::from_item(other), None, "{other}");
        }
    }

    #[test]
    fn defaults_match_upstream() {
        let c = config();
        assert!(c.bunny_hoppers_enabled);
        assert_eq!(c.bunny_hoppers_fall_damage_multiplier, 0.0);
        assert_eq!(c.bunny_hoppers_safe_fall_distance_bonus, 10.0);
        assert_eq!(c.bunny_hoppers_jump_strength_bonus, 0.40);
        assert!(c.feral_claws_enabled);
        assert_eq!(c.feral_claws_attack_speed_bonus, 0.30);
        assert!(c.shock_pendant_enabled);
        assert!(c.shock_pendant_cancel_lightning_damage);
    }

    #[test]
    fn config_parses_and_clamps() {
        let c = parse_config(
            "# server settings\n\
             bunny_hoppers_enabled = false\n\
             bunny_hoppers_safe_fall_distance_bonus = 5000\n\
             bunny_hoppers_jump_strength_bonus = -100\n\
             bunny_hoppers_fall_damage_multiplier = -5\n\
             feral_claws_attack_speed_bonus = 2.5\n\
             shock_pendant_cancel_lightning_damage = false\n\
             unknown = 9\n",
        );
        assert!(!c.bunny_hoppers_enabled);
        assert_eq!(c.bunny_hoppers_safe_fall_distance_bonus, 1024.0);
        assert_eq!(c.bunny_hoppers_jump_strength_bonus, -1.0);
        assert_eq!(c.bunny_hoppers_fall_damage_multiplier, -5.0);
        assert_eq!(c.feral_claws_attack_speed_bonus, 2.5);
        assert!(!c.shock_pendant_cancel_lightning_damage);

        let d = parse_config(
            "bunny_hoppers_enabled = maybe\n\
             bunny_hoppers_safe_fall_distance_bonus = nope\n\
             feral_claws_attack_speed_bonus = -100\n",
        );
        assert!(d.bunny_hoppers_enabled);
        assert_eq!(d.bunny_hoppers_safe_fall_distance_bonus, 10.0);
        assert_eq!(d.feral_claws_attack_speed_bonus, -1.0);
    }

    #[test]
    fn config_round_trips() {
        let c = ServerConfig::default();
        assert_eq!(parse_config(&format_config(&c)), c);
        let c = parse_config(
            "bunny_hoppers_enabled = false\n\
             feral_claws_attack_speed_bonus = 0.75\n\
             shock_pendant_enabled = false\n",
        );
        assert_eq!(parse_config(&format_config(&c)), c);
    }

    #[test]
    fn feral_claws_adds_attack_speed() {
        let worn = worn([Artifact::FeralClaws]);
        let effects = attribute_effects(&worn, &config());
        let attack = effect(&effects, "minecraft:attack_speed");
        assert!(attack.present);
        assert_eq!(attack.id, "artifacts:feral_claws/attack_speed");
        assert_eq!(attack.amount, 0.30);
        assert_eq!(attack.operation, Operation::AddMultipliedBase);
    }

    #[test]
    fn bunny_hoppers_adds_three_modifiers() {
        let worn = worn([Artifact::BunnyHoppers]);
        let effects = attribute_effects(&worn, &config());

        let jump = effect(&effects, "minecraft:jump_strength");
        assert!(jump.present);
        assert_eq!(jump.id, "artifacts:bunny_hoppers/jump_strength");
        assert_eq!(jump.amount, 0.40);
        assert_eq!(jump.operation, Operation::AddMultipliedBase);

        let fall = effect(&effects, "minecraft:fall_damage_multiplier");
        assert!(fall.present);
        assert_eq!(fall.amount, 0.0);
        assert_eq!(fall.operation, Operation::AddMultipliedBase);

        let safe = effect(&effects, "minecraft:safe_fall_distance");
        assert!(safe.present);
        assert_eq!(safe.amount, 10.0);
        assert_eq!(safe.operation, Operation::AddValue);
    }

    #[test]
    fn an_empty_loadout_removes_every_modifier() {
        let effects = attribute_effects(&Loadout::new(), &config());
        assert_eq!(effects.len(), 4);
        assert!(effects.iter().all(|effect| !effect.present));
    }

    #[test]
    fn disabled_artifacts_are_absent() {
        let c = parse_config(
            "bunny_hoppers_enabled = false\n\
             feral_claws_enabled = false\n\
             shock_pendant_enabled = false\n",
        );
        let worn = worn(Artifact::ALL);
        let effects = attribute_effects(&worn, &c);
        assert!(effects.iter().all(|effect| !effect.present));
    }

    #[test]
    fn modifier_ids_are_unique() {
        let worn = worn(Artifact::ALL);
        let effects = attribute_effects(&worn, &config());
        let mut ids: Vec<&str> = effects.iter().map(|effect| effect.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), effects.len());
    }

    #[test]
    fn operations_have_the_pin_names() {
        assert_eq!(Operation::AddValue.as_str(), "add_value");
        assert_eq!(Operation::AddMultipliedBase.as_str(), "add_multiplied_base");
    }

    #[test]
    fn lightning_tag_is_exact() {
        assert!(is_lightning("minecraft:lightning_bolt"));
        for other in [
            "minecraft:lightning",
            "lightning_bolt",
            "minecraft:generic",
            "minecraft:on_fire",
            "",
        ] {
            assert!(!is_lightning(other), "{other}");
        }
    }

    #[test]
    fn shock_pendant_blocks_lightning() {
        let worn = worn([Artifact::ShockPendant]);
        assert!(blocks_damage(
            &worn,
            "minecraft:player",
            "minecraft:lightning_bolt",
            &config()
        ));
    }

    #[test]
    fn shock_pendant_needs_the_item() {
        assert!(!blocks_damage(
            &Loadout::new(),
            "minecraft:player",
            "minecraft:lightning_bolt",
            &config()
        ));
    }

    #[test]
    fn shock_pendant_respects_both_switches() {
        let worn = worn([Artifact::ShockPendant]);
        let off = parse_config("shock_pendant_cancel_lightning_damage = false\n");
        assert!(!blocks_damage(
            &worn,
            "minecraft:player",
            "minecraft:lightning_bolt",
            &off
        ));
        let disabled = parse_config("shock_pendant_enabled = false\n");
        assert!(!blocks_damage(
            &worn,
            "minecraft:player",
            "minecraft:lightning_bolt",
            &disabled
        ));
    }

    #[test]
    fn shock_pendant_ignores_other_damage() {
        let worn = worn([Artifact::ShockPendant]);
        for other in [
            "minecraft:fall",
            "minecraft:on_fire",
            "minecraft:in_fire",
            "minecraft:generic",
        ] {
            assert!(
                !blocks_damage(&worn, "minecraft:player", other, &config()),
                "{other}"
            );
        }
    }

    #[test]
    fn shock_pendant_does_not_protect_mobs() {
        let worn = worn([Artifact::ShockPendant]);
        assert!(!blocks_damage(
            &worn,
            "minecraft:zombie",
            "minecraft:lightning_bolt",
            &config()
        ));
    }

    #[test]
    fn bonus_values_can_be_negative_to_cancel() {
        let c = parse_config(
            "bunny_hoppers_fall_damage_multiplier = -1\n\
             feral_claws_attack_speed_bonus = -1\n",
        );
        let worn = worn([Artifact::BunnyHoppers, Artifact::FeralClaws]);
        let effects = attribute_effects(&worn, &c);
        assert_eq!(
            effect(&effects, "minecraft:fall_damage_multiplier").amount,
            -1.0
        );
        assert_eq!(effect(&effects, "minecraft:attack_speed").amount, -1.0);
    }

    #[test]
    fn the_worn_slot_is_the_inventory_off_hand() {
        assert_eq!(WORN_SLOT, 40, "Inventory's off-hand number");
        assert!(is_worn_slot(WORN_SLOT));
        assert!(!is_worn_slot(39), "head is not worn");
        assert!(!is_worn_slot(41), "body is not worn");
    }

    #[test]
    fn only_the_off_hand_is_worn() {
        let worn = worn_from_slots([(WORN_SLOT, Artifact::FeralClaws, 1)]);
        assert!(worn.has(Artifact::FeralClaws), "the off hand is worn");

        // 0 to 35 are the main inventory (0 to 8 the hotbar), 36 to 39 the
        // armour, 41 body and 42 saddle: carried, not worn.
        for slot in [0, 1, 8, 9, 35, 36, 37, 38, 39, 41, 42] {
            let carried = worn_from_slots([(slot, Artifact::FeralClaws, 1)]);
            assert!(
                !carried.has(Artifact::FeralClaws),
                "slot {slot} is carried, not worn"
            );
        }
    }

    #[test]
    fn a_worn_stack_counts_once_and_a_zero_count_is_not_worn() {
        let worn = worn_from_slots([
            (WORN_SLOT, Artifact::BunnyHoppers, 1),
            (WORN_SLOT, Artifact::BunnyHoppers, 1),
        ]);
        assert!(worn.has(Artifact::BunnyHoppers));
        assert!(!worn.has(Artifact::FeralClaws));

        let empty = worn_from_slots([(WORN_SLOT, Artifact::BunnyHoppers, 0)]);
        assert!(!empty.has(Artifact::BunnyHoppers));
    }

    #[test]
    fn a_carried_artifact_grants_nothing_while_the_worn_one_does() {
        let carried = worn_from_slots([(0, Artifact::FeralClaws, 1)]);
        let effects = attribute_effects(&carried, &config());
        assert!(
            effects.iter().all(|effect| !effect.present),
            "a carried artifact changes no attribute"
        );

        let worn = worn_from_slots([(WORN_SLOT, Artifact::FeralClaws, 1)]);
        let effects = attribute_effects(&worn, &config());
        assert!(effect(&effects, "minecraft:attack_speed").present);
    }

    #[test]
    fn the_pendant_only_protects_when_worn() {
        let carried = worn_from_slots([(0, Artifact::ShockPendant, 1)]);
        assert!(!blocks_damage(
            &carried,
            "minecraft:player",
            "minecraft:lightning_bolt",
            &config()
        ));
        let worn = worn_from_slots([(WORN_SLOT, Artifact::ShockPendant, 1)]);
        assert!(blocks_damage(
            &worn,
            "minecraft:player",
            "minecraft:lightning_bolt",
            &config()
        ));
    }
}
