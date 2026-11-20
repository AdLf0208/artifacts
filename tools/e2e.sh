#!/usr/bin/env bash
# In-game check for the Artifacts port (0.2.1, the off hand is the worn slot).
#
# Upstream defaults (ItemConfigs.java:152-173,491-501,880-903):
# - Bunny Hoppers: jump strength +0.40 (ADD_MULTIPLIED_BASE, 0.42 -> 0.588),
#   safe fall distance +10 (ADD_VALUE, 3 -> 13), fall damage multiplier +0
#   (ADD_MULTIPLIED_BASE, unchanged at 1).
# - Feral Claws: attack speed +0.30 (ADD_MULTIPLIED_BASE, 4 -> 5.2).
# - Shock Pendant: lightning damage is cancelled while worn.
#
# Slice 3's differential is the first effect phase: the same Feral Claws in the
# main inventory changes nothing (4.0), and worn in the off hand it gives 5.2.
# Upstream only counts a *worn* artifact, so the 0.2.0 package — which scanned
# the whole inventory — fails that phase. Every other effect phase wears its
# artifact through the mod's own right click (`use_item`) first.
#
# MODE=control turns all three artifacts off in `config/artifacts.conf`: the
# same worn items leave every attribute at its baseline, the same fall hurts and
# the same lightning hit hurts. That is the config differential. The package
# differential is a run with `GAME=$CONTROL_GAME` (the same Nightly without the
# package), which fails at the first item and exits 1.
#
# SELFTEST=1 keeps the default setup but asserts the worn claws' +30 % is
# absent, so the run must fail: that is the counterfactual proving the check can
# fail.
#
# The package carries no recipe (upstream ships none for these three items; its
# only recipes are eternal_steak's three cooking recipes), so the item family is
# proved by give -> read back -> effect, an unknown id is proved refused, and
# the three items' effects are proved from the off hand.
#
# SDK layout (0.2.0+): GAME must be a **launcher-built** horizon-client carrying
# this package (`horizon-launcher native-build <port> … --trust-native-code`),
# because the mod's code is compiled into the game. The package also carries the
# content, so no `--dev-mod` is needed; DEV_MOD=1 adds the delivered
# `artifacts.hmod` as one anyway (for a build that only carries the code).
set -euo pipefail

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/artifacts
# The control binary is shared at the campaign pin it was published for; it must speak the
# same protocol as GAME, so point CTL at a horizon-ctl built from the game's own build when
# one is published for it. The game is the launcher build carrying this package.
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}
SOCK=${SOCK:-/tmp/opencode/artifacts-e2e.sock}
MODE=${MODE:-default}
SELFTEST=${SELFTEST:-0}
# DEV_MOD=1 also hands the delivered package to the game with `--dev-mod` (a
# launcher build already carries it, so this is off by default).
DEV_MOD=${DEV_MOD:-0}
WORLD=ARTIFACTSE2E

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/horizon-mod.toml" ] || { echo "missing package manifest: $PORT/horizon-mod.toml" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/artifacts-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$SCRATCH/config" "$SCRATCH/game/config"
rm -f "$SOCK"
# The game folder is the working directory: keep saves/options out of the port.
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

# `/attribute @s <id> get` prints the live server value. `ctl command` runs it
# on the integrated server as the owner and returns the reply lines, so a slow
# server tick cannot push the reply out of a chat window.
attribute() {
    ctl command "attribute @s $1 get" \
        | jq -r '.chat[]?' \
        | grep -oE 'is -?[0-9]+(\.[0-9]+)?' | head -1 | awk '{print $2}' \
        || true
}
expect_attribute() {
    local got
    got=$(attribute "$1")
    if [ -z "$got" ] \
        || ! awk -v got="$got" -v want="$2" -v tol="$3" \
            'BEGIN { d = got - want; if (d < 0) d = -d; exit !(d <= tol) }'; then
        ctl state | jq -c '.player' >&2 || true
        fail "$4: expected $1 = $2, got ${got:-nothing}"
    fi
    echo "$1 = $got ($4)"
}
health() { ctl effects | jq -r '.health'; }
expect_health() {
    local got
    got=$(health | awk '{printf "%.0f", $1}')
    if [ "$got" != "$1" ]; then
        ctl effects >&2 || true
        fail "$2: expected health $1, got ${got:-nothing}"
    fi
    echo "health $got ($2)"
}
# The fall test asserts the differential (a hurt without the artifact, none
# with it); the exact amount depends on the landing height.
expect_hurt() {
    local got
    got=$(health | awk '{printf "%.0f", $1}')
    if [ "${got:-0}" -le 0 ] || [ "${got:-0}" -ge 20 ]; then
        ctl effects >&2 || true
        fail "$1: expected a hurt (health 1..19), got ${got:-nothing}"
    fi
    echo "health $got ($1: hurt)"
}
item_count() { ctl inventory | jq -r --arg i "$1" '.counts[$i] // 0'; }
heal_full() {
    ctl command "effect give @s minecraft:instant_health 1 10" >/dev/null
    ctl wait 22
}
# `/damage` with an explicit type; a cancelled hit makes the command report
# "invulnerable", which is the protected case and not a script failure.
damage() {
    ctl command "damage @s $1 minecraft:$2" >/dev/null 2>&1 || true
    ctl wait 22
}
fall_from_10() {
    ctl command "tp @s 0.5 211 0.5" >/dev/null \
        || fail "cannot teleport for the fall test"
    ctl wait 5
    echo "fall-start: $(ctl state | jq -c '{dimension: .dimension, position: .player.position, health: .player.health, on_ground: .player.on_ground}')"
    ctl wait 40
    echo "fall-end: $(ctl state | jq -c '{dimension: .dimension, position: .player.position, health: .player.health, on_ground: .player.on_ground}')"
}

# `ctl inventory` names the off hand with the documented slot 504; a missing or
# renamed field fails loudly instead of letting an "empty" assertion pass.
offhand_item() { ctl inventory | jq -r '.offhand.item // ""'; }
offhand_slot() { ctl inventory | jq -r '.offhand.slot // ""'; }
# The client's own copy can lag the server by a tick, so `hold_item` selects and
# confirms the held stack before any click: a give/select race must never make
# the use a no-op.
hold_item() {
    local item=$1 i held err=""
    for i in 1 2 3 4 5 6 7 8; do
        err=$(ctl select "$item" 2>&1 >/dev/null) || true
        ctl wait 3 >/dev/null
        held=$(ctl inventory | jq -r '.held.item // ""')
        [ "$held" = "$item" ] && return 0
    done
    fail "cannot hold $item: ${err:-held '$held'}"
}
# `/clear` empties the main inventory, the armour and the off hand; the off hand
# must really be empty afterwards or every "worn" phase below would be lying.
clear_all() {
    ctl command "clear @s" >/dev/null
    ctl wait 6
    local left
    left=$(offhand_item)
    [ -z "$left" ] || fail "the off hand still holds '${left}' after /clear"
}
# wear ITEM: the mod's own equip path — give one, hold it, right-click it into
# the empty off hand, and require the off hand to hold it afterwards.
wear() {
    local item=$1 got
    clear_all
    ctl give "$item" 1 >/dev/null || fail "cannot give $item"
    ctl wait 4
    hold_item "$item"
    ctl use-item
    ctl wait 6
    got=$(offhand_item)
    [ "$got" = "$item" ] \
        || fail "right-click did not wear $item: off hand is '${got:-empty}'"
    echo "worn: $got"
}

cleanup() {
    local status=$?
    ctl quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" 2>/dev/null || true
    # Last line of every run: `check-e2e-logs.py` reads it as the verdict.
    echo "EXIT:$status"
}
trap cleanup EXIT

launch() { # launch [WORLD]
    local world=${1:-$WORLD}
    rm -f "$SOCK"
    local args=(--game "$GAME" --world "$world" --seed 20261105 --creative --headless --wait
                --resources "$RES")
    if [ "$DEV_MOD" = 1 ]; then
        args+=(--dev-mod "$PORT/artifacts.hmod")
    fi
    "$CTL" --socket "$SOCK" launch "${args[@]}" >"$SCRATCH/game-console.log" 2>&1
    ctl wait-ready 120
    ctl wait 30
    ctl command "tp @s 0.5 201 0.5" >/dev/null
    ctl wait 5
    ctl command "fill -4 200 -4 4 200 4 minecraft:stone" >/dev/null \
        || fail "cannot build the sky platform floor"
    ctl command "tp @s 0.5 201 0.5" >/dev/null
    ctl wait 10
    ctl command "kill @e[type=minecraft:ender_dragon]" >/dev/null 2>&1 || true
    ctl command "kill @e[type=minecraft:enderman]" >/dev/null 2>&1 || true
    ctl wait 3
}

# The SDK's config service keeps the mod's files in the game's `config/` folder,
# which under this scratch home is `$XDG_CONFIG_HOME` (probed: the package wrote
# its defaults there; `$HORIZON_HOME/config` is not read).
if [ "$MODE" = control ]; then
    cat > "$SCRATCH/config/artifacts.conf" <<'EOF'
bunny_hoppers_enabled = false
bunny_hoppers_fall_damage_multiplier = 0
bunny_hoppers_safe_fall_distance_bonus = 10
bunny_hoppers_jump_strength_bonus = 0.4
feral_claws_enabled = false
feral_claws_attack_speed_bonus = 0.3
shock_pendant_enabled = false
shock_pendant_cancel_lightning_damage = true
EOF
    echo "control: all three artifacts disabled"
else
    cat > "$SCRATCH/config/artifacts.conf" <<'EOF'
bunny_hoppers_enabled = true
bunny_hoppers_fall_damage_multiplier = 0
bunny_hoppers_safe_fall_distance_bonus = 10
bunny_hoppers_jump_strength_bonus = 0.4
feral_claws_enabled = true
feral_claws_attack_speed_bonus = 0.3
shock_pendant_enabled = true
shock_pendant_cancel_lightning_damage = true
EOF
    echo "default: upstream defaults, all three enabled"
fi

echo "== launch (creative, Overworld sky platform, launcher-built game) =="
launch

echo "== baseline attributes =="
expect_attribute minecraft:attack_speed 4.0 0.01 "baseline"
expect_attribute minecraft:safe_fall_distance 3.0 0.01 "baseline"
expect_attribute minecraft:jump_strength 0.42 0.01 "baseline"
expect_attribute minecraft:fall_damage_multiplier 1.0 0.01 "baseline"

echo "== every content item id is giveable, an unknown id is refused =="
ITEMS="artifacts:bunny_hoppers artifacts:feral_claws artifacts:shock_pendant"
for item in $ITEMS; do
    ctl give "$item" 1 >/dev/null || fail "the content mod did not provide $item"
done
ctl wait 8
for item in $ITEMS; do
    n=$(item_count "$item")
    [ "$n" -ge 1 ] || fail "$item was not in the inventory after /give"
    echo "giveable: $item x$n"
done
clear_all
# An id the package does not declare must be refused, and nothing may appear.
# A failed vanilla command still answers `ok:true` with the game's message in
# `chat`, so the refusal is read from the reply text.
unknown=$(ctl command "give @s artifacts:not_an_artifact 1" 2>&1) || true
printf '%s' "$unknown" | grep -qi 'unknown item' \
    || fail "an unknown item id was not refused: $unknown"
[ "$(item_count artifacts:not_an_artifact)" = 0 ] \
    || fail "the unknown id entered the inventory"
echo "unknown id refused: artifacts:not_an_artifact"

echo "== a stack of sticks is a control item =="
ctl give minecraft:stick 64 >/dev/null || fail "cannot give sticks"
ctl wait 6
expect_attribute minecraft:attack_speed 4.0 0.01 "64 sticks change nothing"
expect_attribute minecraft:safe_fall_distance 3.0 0.01 "64 sticks change nothing"
clear_all

echo "== Feral Claws: carried is not worn, the off hand is =="
# The same item, two places. Carried in the main inventory it must change
# nothing (upstream only counts a worn artifact); a right click into the off
# hand must then give the full +30 %. The 0.2.0 package fails the carried half.
clear_all
ctl give artifacts:feral_claws 1 >/dev/null || fail "cannot give the feral claws"
ctl wait 6
[ -z "$(offhand_item)" ] \
    || fail "the carried phase already has something in the off hand: '$(offhand_item)'"
if [ "$MODE" = control ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "control: a carried claw is off"
else
    expect_attribute minecraft:attack_speed 4.0 0.01 "merely carried: no bonus"
fi
echo "carried: attack_speed 4.0 (the inventory is not a worn slot)"

hold_item artifacts:feral_claws
ctl use-item
ctl wait 6
offhand=$(offhand_item)
[ "$offhand" = "artifacts:feral_claws" ] \
    || fail "right-click equip left the off hand as '${offhand:-empty}'"
[ "$(offhand_slot)" = 504 ] \
    || fail "ctl inventory reports the off hand as slot '$(offhand_slot)', not 504"
echo "worn: offhand $offhand (slot $(offhand_slot))"
if [ "$MODE" = control ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "control: worn claws are off"
elif [ "$SELFTEST" = 1 ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "selftest: the claws' bonus is asserted absent (must FAIL)"
else
    expect_attribute minecraft:attack_speed 5.2 0.01 "worn: the same claws give +30%"
fi
# A duplicate in the inventory adds nothing: one worn slot per kind.
ctl give artifacts:feral_claws 1 >/dev/null
ctl wait 6
if [ "$MODE" = control ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "control: two claws are off"
else
    expect_attribute minecraft:attack_speed 5.2 0.01 "a carried duplicate adds nothing"
fi
clear_all
expect_attribute minecraft:attack_speed 4.0 0.01 "claws taken off"

echo "== Bunny Hoppers: safe fall +10, jump +40%, multiplier unchanged =="
wear artifacts:bunny_hoppers
if [ "$MODE" = control ]; then
    expect_attribute minecraft:safe_fall_distance 3.0 0.01 "control: hoppers are off"
    expect_attribute minecraft:jump_strength 0.42 0.01 "control: hoppers are off"
    expect_attribute minecraft:fall_damage_multiplier 1.0 0.01 "control: hoppers are off"
else
    expect_attribute minecraft:safe_fall_distance 13.0 0.01 "safe fall +10"
    expect_attribute minecraft:jump_strength 0.588 0.01 "jump strength +40%"
    expect_attribute minecraft:fall_damage_multiplier 1.0 0.01 "fall multiplier unchanged"
fi
clear_all
expect_attribute minecraft:safe_fall_distance 3.0 0.01 "hoppers taken off"

echo "== survival: lightning and fall hits =="
ctl command "gamerule natural_health_regeneration false" >/dev/null
ctl command "gamerule spawn_mobs false" >/dev/null
ctl command "difficulty normal" >/dev/null
ctl command "gamemode survival" >/dev/null
ctl command "kill @e[type=minecraft:enderman]" >/dev/null 2>&1 || true
ctl wait 5
heal_full

echo "== Shock Pendant: a lightning hit =="
expect_health 20 "survival baseline"
damage 5 lightning_bolt
if [ "$MODE" = control ]; then
    expect_health 15 "control: lightning hurts (pendant off)"
else
    expect_health 15 "no pendant: lightning hurts"
fi
heal_full
wear artifacts:shock_pendant
expect_health 20 "full health with the pendant worn"
damage 5 lightning_bolt
if [ "$MODE" = control ]; then
    expect_health 15 "control: lightning still hurts with the pendant off"
else
    expect_health 20 "the worn pendant cancels lightning"
fi
heal_full
damage 5 fall
if [ "$MODE" = control ]; then
    expect_health 15 "control: a fall hit hurts"
else
    expect_health 15 "the pendant does not cancel a fall hit"
fi
heal_full
clear_all
damage 5 lightning_bolt
expect_health 15 "lightning hurts after the pendant is taken off"

echo "== Bunny Hoppers: a 10-block fall =="
heal_full
fall_from_10
if [ "$MODE" = control ]; then
    expect_hurt "control: the 10-block fall hurts"
else
    expect_hurt "no hoppers: the 10-block fall hurts"
fi
heal_full
wear artifacts:bunny_hoppers
heal_full
fall_from_10
if [ "$MODE" = control ]; then
    expect_hurt "control: the hoppers are off and the fall hurts"
else
    expect_health 20 "the worn hoppers make a 10-block fall safe"
fi
heal_full
clear_all
fall_from_10
expect_hurt "the fall hurts after the hoppers are taken off"

echo "== right click equip: a non-artifact is never moved, an occupied off hand is never replaced =="
# A freshly cleared player has an empty off hand; fail if the precondition does
# not hold, so the control below cannot pass vacuously.
clear_all
offhand=$(offhand_item)
[ -z "$offhand" ] || fail "the off hand is not empty before the equip phase: '${offhand}'"

ctl give minecraft:stick 1 >/dev/null || fail "cannot give the stick control"
hold_item minecraft:stick
ctl use-item
ctl wait 6
offhand=$(offhand_item)
[ -z "$offhand" ] || fail "a non-artifact was equipped: offhand '${offhand}'"
[ "$(ctl inventory | jq -r '.held.item // ""')" = "minecraft:stick" ] \
    || fail "the stick left the main hand"
echo "non-artifact control: offhand stayed empty, the stick stayed in the main hand"

# An occupied off hand is never replaced, and the refused item stays in hand.
wear artifacts:feral_claws
ctl give artifacts:bunny_hoppers 1 >/dev/null || fail "cannot give the bunny hoppers"
hold_item artifacts:bunny_hoppers
ctl use-item
ctl wait 6
[ "$(offhand_item)" = "artifacts:feral_claws" ] \
    || fail "an occupied off hand was replaced: offhand '$(offhand_item)'"
[ "$(ctl inventory | jq -r '.held.item // ""')" = "artifacts:bunny_hoppers" ] \
    || fail "the refused item left the main hand"
echo "occupied off hand: kept artifacts:feral_claws, the bunny hoppers stayed in the main hand"

if [ "$MODE" = "control" ]; then
    echo "CONTROL PASS: all three artifacts disabled left attack speed 4.0, safe fall 3.0, jump 0.42; the same lightning hit and the same 10-block fall hurt, while the off-hand equip controls still held"
else
    echo "PASS: the three ids are giveable and an unknown id is refused, a carried Feral Claws gives nothing while the same claws worn in the off hand (slot 504) give attack speed 5.2 once, hoppers give safe fall 13 and jump 0.588, the pendant cancels a lightning hit but not a fall hit, and the hoppers make a 10-block fall safe"
fi
