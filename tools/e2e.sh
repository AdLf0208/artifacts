#!/usr/bin/env bash
# In-game check for the Artifacts slice-1 items.
#
# Upstream defaults (ItemConfigs.java:152-173,491-501,880-903):
# - Bunny Hoppers: jump strength +0.40 (ADD_MULTIPLIED_BASE, 0.42 -> 0.588),
#   safe fall distance +10 (ADD_VALUE, 3 -> 13), fall damage multiplier +0
#   (ADD_MULTIPLIED_BASE, unchanged at 1).
# - Feral Claws: attack speed +0.30 (ADD_MULTIPLIED_BASE, 4 -> 5.2).
# - Shock Pendant: lightning damage is cancelled while carried.
#
# Default run: the three ids are giveable, a stick changes nothing, the
# attribute table is exact, a duplicate artifact adds nothing, a 10-block fall
# hurts without the hoppers and not with them, and a lightning hit hurts
# without the pendant and not with it (while a fall hit still hurts).
#
# MODE=control turns all three artifacts off in `config/artifacts.conf`: the
# same items leave every attribute at its baseline, the same fall hurts and the
# same lightning hit hurts. That is the differential: an implementation that
# ignored the config or the inventory fails one of the two runs.
#
# EMBEDDED=1 launches the game with **only the native bundle** (no `--dev-mod`):
# the content comes from the bundle's embedded copy (`mixins/artifacts/data/`,
# the AE2 pattern). The run checks the file the helper wrote is byte-identical
# to `content/artifacts.hmod`, and CONTROL=1 (default with EMBEDDED) first
# launches the same binary with `.horizon-cache` unusable, where the three item
# ids must be unknown.
set -euo pipefail

CLONE=${CAMPAIGN_ROOT:-$(cd "$(dirname "$0")/../../.." && pwd)}
PORT=$CLONE/ports/artifacts
# The control binary is shared at the campaign pin; the game is this port's build.
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}
GAME=${GAME:-$PORT/target/horizon-cargo/fast/horizon-client}
RES=${RES:-${HORIZON_RES:-${HORIZON_RES:-$HOME/.local/horizon-client/26.3}}}
SOCK=${SOCK:-/tmp/opencode/artifacts-e2e.sock}
MODE=${MODE:-default}
# SELFTEST=1 keeps the default setup but asserts the claws' +30 % is absent, so
# the run must fail: that is the counterfactual proving the check can fail.
SELFTEST=${SELFTEST:-0}
EMBEDDED=${EMBEDDED:-0}
CONTROL=${CONTROL:-$EMBEDDED}
WORLD=ARTIFACTSE2E
CONTROL_WORLD=ARTIFACTSControl

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }
[ -f "$PORT/content/horizon-mod.toml" ] || { echo "missing content mod: $PORT/content" >&2; exit 1; }

SCRATCH=$(mktemp -d /tmp/opencode/artifacts-home.XXXXXX)
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$SCRATCH/config"
rm -f "$SOCK"
# The game folder is the working directory: keep saves/options out of the port.
cd "$SCRATCH"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

# `/attribute @s <id> get` prints the live server value. `ctl command` runs it
# on the integrated server as the owner and returns the reply lines, so a slow
# server tick cannot push the reply out of a chat window (the old
# chat/messages form aborted under `set -e` when that happened).
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
clear_item() {
    ctl chat "/clear @s $1" >/dev/null
    ctl wait 6
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
    if [ "$EMBEDDED" = 1 ]; then
        echo "  launch: no --dev-mod; the bundle carries its own content" >&2
    else
        args+=(--dev-mod "$PORT/content")
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

# EMBEDDED=1: the bundle must have written its content out at startup, byte for
# byte the delivered content mod.
embedded_content_check() {
    local cache=$SCRATCH/.horizon-cache/artifacts-embedded-content.hmod
    [ -f "$cache" ] || fail "the bundle did not write its embedded content to $cache"
    cmp -s "$cache" "$PORT/content/artifacts.hmod" \
        || fail "the embedded content differs from content/artifacts.hmod"
    echo "  embedded content: $(wc -c < "$cache" | tr -d ' ') B at $cache, identical to content/artifacts.hmod"
}

# The control for EMBEDDED=1: the same binary (every hook is in it) with the
# embedded content made unavailable — `.horizon-cache` is a plain file, so the
# helper can neither compare nor write its content file and falls back to
# `Continue`. All three ids must then be unknown; if they resolve, the run
# proves nothing about the embedded file.
control_leg() {
    echo "== control: the same game with no embedded content file =="
    rm -rf "$SCRATCH/.horizon-cache"
    : > "$SCRATCH/.horizon-cache"
    launch "$CONTROL_WORLD"
    ctl command "item replace entity @s hotbar.0 with artifacts:bunny_hoppers 1" >/dev/null 2>&1 || true
    ctl wait 4
    ctl inventory \
        | jq -e '[.hotbar[], .main[] | select(.item != null and (.item | startswith("artifacts:")))] | length == 0' >/dev/null \
        || fail "the control game holds an artifacts item, so the content did not come from the embedded file"
    ctl command "setblock 0 201 0 artifacts:bunny_hoppers" >/dev/null 2>&1 || true
    ctl wait 4
    local name
    name=$(block_name 0 201 0)
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the item id was known"
    [ -f "$SCRATCH/.horizon-cache" ] || fail "the control's .horizon-cache was replaced: the helper wrote its content"
    echo "  without the embedded file: no artifacts item, the block stayed $name"
    ctl quit >/dev/null 2>&1 || true
    rm -f "$SCRATCH/.horizon-cache"
}

echo "== launch (creative, Overworld sky platform, $([ "$EMBEDDED" = 1 ] && echo 'one bundle, no --dev-mod' || echo 'content/dev-mod')) =="
if [ "$CONTROL" = 1 ]; then control_leg; fi
launch
if [ "$EMBEDDED" = 1 ]; then embedded_content_check; fi

echo "== baseline attributes =="
expect_attribute minecraft:attack_speed 4.0 0.01 "baseline"
expect_attribute minecraft:safe_fall_distance 3.0 0.01 "baseline"
expect_attribute minecraft:jump_strength 0.42 0.01 "baseline"
expect_attribute minecraft:fall_damage_multiplier 1.0 0.01 "baseline"

echo "== every content item id is giveable =="
ITEMS="artifacts:bunny_hoppers artifacts:feral_claws artifacts:shock_pendant"
for item in $ITEMS; do
    ctl give "$item" 1 >/dev/null || fail "the content mod did not provide $item"
done
ctl wait 8
for item in $ITEMS; do
    n=$(item_count "$item")
    [ "$n" -ge 1 ] || fail "$item was not in the inventory after /give"
    echo "giveable: $item"
done
ctl chat "/clear @s" >/dev/null
ctl wait 8

echo "== a stack of sticks is a control item =="
ctl give minecraft:stick 64 >/dev/null || fail "cannot give sticks"
ctl wait 6
expect_attribute minecraft:attack_speed 4.0 0.01 "64 sticks change nothing"
expect_attribute minecraft:safe_fall_distance 3.0 0.01 "64 sticks change nothing"
clear_item minecraft:stick

echo "== Feral Claws: attack speed +30% =="
ctl give artifacts:feral_claws 1 >/dev/null || fail "cannot give the feral claws"
ctl wait 6
if [ "$MODE" = control ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "control: claws are off"
elif [ "$SELFTEST" = 1 ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "selftest: the claws' bonus is asserted absent (must FAIL)"
else
    expect_attribute minecraft:attack_speed 5.2 0.01 "one feral claws (+30%)"
fi
ctl give artifacts:feral_claws 1 >/dev/null
ctl wait 6
if [ "$MODE" = control ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "control: two claws are off"
else
    expect_attribute minecraft:attack_speed 5.2 0.01 "a duplicate adds nothing"
fi
clear_item artifacts:feral_claws
expect_attribute minecraft:attack_speed 4.0 0.01 "claws removed"

echo "== Bunny Hoppers: safe fall +10, jump +40%, multiplier unchanged =="
ctl give artifacts:bunny_hoppers 1 >/dev/null || fail "cannot give the bunny hoppers"
ctl wait 6
if [ "$MODE" = control ]; then
    expect_attribute minecraft:safe_fall_distance 3.0 0.01 "control: hoppers are off"
    expect_attribute minecraft:jump_strength 0.42 0.01 "control: hoppers are off"
    expect_attribute minecraft:fall_damage_multiplier 1.0 0.01 "control: hoppers are off"
else
    expect_attribute minecraft:safe_fall_distance 13.0 0.01 "safe fall +10"
    expect_attribute minecraft:jump_strength 0.588 0.01 "jump strength +40%"
    expect_attribute minecraft:fall_damage_multiplier 1.0 0.01 "fall multiplier unchanged"
fi
clear_item artifacts:bunny_hoppers
expect_attribute minecraft:safe_fall_distance 3.0 0.01 "hoppers removed"

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
ctl give artifacts:shock_pendant 1 >/dev/null || fail "cannot give the shock pendant"
ctl wait 6
expect_health 20 "full health with the pendant"
damage 5 lightning_bolt
if [ "$MODE" = control ]; then
    expect_health 15 "control: lightning still hurts with the pendant off"
else
    expect_health 20 "the pendant cancels lightning"
fi
heal_full
damage 5 fall
if [ "$MODE" = control ]; then
    expect_health 15 "control: a fall hit hurts"
else
    expect_health 15 "the pendant does not cancel a fall hit"
fi
heal_full
clear_item artifacts:shock_pendant
damage 5 lightning_bolt
expect_health 15 "lightning hurts after the pendant is removed"

echo "== Bunny Hoppers: a 10-block fall =="
heal_full
fall_from_10
if [ "$MODE" = control ]; then
    expect_hurt "control: the 10-block fall hurts"
else
    expect_hurt "no hoppers: the 10-block fall hurts"
fi
heal_full
ctl give artifacts:bunny_hoppers 1 >/dev/null || fail "cannot give the bunny hoppers"
ctl wait 6
heal_full
fall_from_10
if [ "$MODE" = control ]; then
    expect_hurt "control: the hoppers are off and the fall hurts"
else
    expect_health 20 "the hoppers make a 10-block fall safe"
fi
heal_full
clear_item artifacts:bunny_hoppers
fall_from_10
expect_hurt "the fall hurts after the hoppers are removed"

echo "== right click equip: an artifact moves to an empty off hand, a non-artifact does not =="
# `ctl inventory` carries the documented off-hand slot (504). Echo its shape,
# and require the field, so a missing/renamed field fails loudly instead of
# letting an "empty" assertion pass vacuously. The client's own copy can lag
# the server by a tick, so `hold_item` selects and confirms the held stack
# before any click: a give/select race must never make the use a no-op.
offhand_field=$(ctl inventory | jq -c '.offhand')
echo "offhand field: $offhand_field"
[ "$(printf '%s' "$offhand_field" | jq -r 'has("item")')" = "true" ] \
    || fail "ctl inventory has no offhand.item field: $offhand_field"
offhand_item() { ctl inventory | jq -r '.offhand.item // ""'; }
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

# A freshly launched player has an empty off hand; the earlier phases only
# clear their own items. Fail if the precondition does not hold.
ctl command "clear @s" >/dev/null
ctl wait 6
offhand=$(offhand_item)
[ -z "$offhand" ] || fail "the off hand is not empty before the equip phase: '${offhand}'"

# Control A: a non-artifact is never moved. The same click sequence with a
# stick must leave the off hand empty; a port that equips any item fails here.
ctl give minecraft:stick 1 >/dev/null || fail "cannot give the stick control"
hold_item minecraft:stick
ctl use-item
ctl wait 6
offhand=$(offhand_item)
[ -z "$offhand" ] || fail "a non-artifact was equipped: offhand '${offhand}'"
[ "$(ctl inventory | jq -r '.held.item // ""')" = "minecraft:stick" ] \
    || fail "the stick left the main hand"
echo "non-artifact control: offhand stayed empty, the stick stayed in the main hand"

# Positive: an empty off hand takes the artifact, and the main hand gives it
# up. (The occupied-off-hand branch is a runtime unit test, not this scene:
# there is no reliable server-side off-hand fixture on this pin.)
ctl give artifacts:feral_claws 1 >/dev/null || fail "cannot give the feral claws"
hold_item artifacts:feral_claws
ctl use-item
ctl wait 6
offhand=$(offhand_item)
[ "$offhand" = "artifacts:feral_claws" ] \
    || fail "right-click equip left the off hand as '${offhand:-empty}'"
held=$(ctl inventory | jq -r '.held.item // ""')
[ "$held" != "artifacts:feral_claws" ] \
    || fail "the claws are still in the main hand after the equip"
if [ "$MODE" = control ]; then
    expect_attribute minecraft:attack_speed 4.0 0.01 "control: equipped claws are off"
    echo "right-click equip (control): offhand $offhand, main hand '${held:-empty}'"
else
    expect_attribute minecraft:attack_speed 5.2 0.01 "equipped claws still apply"
    echo "right-click equip: offhand $offhand, main hand '${held:-empty}'"
fi

if [ "$MODE" = "control" ]; then
    echo "CONTROL PASS: all three artifacts disabled left attack speed 4.0, safe fall 3.0, jump 0.42; the same lightning hit and the same 10-block fall hurt, while the off-hand equip controls still held"
elif [ "$EMBEDDED" = 1 ]; then
    echo "PASS: one bundle, no --dev-mod: the embedded content gave the three ids (attack speed 5.2 once, safe fall 13, jump 0.588), the pendant cancelled a lightning hit, the hoppers made a 10-block fall safe, and the control showed the ids unknown without the embedded file"
else
    echo "PASS: the three ids are giveable, claws give attack speed 5.2 once, hoppers give safe fall 13 and jump 0.588, the pendant cancels a lightning hit but not a fall hit, and the hoppers make a 10-block fall safe, and right-click moves an artifact to the off hand"
fi
