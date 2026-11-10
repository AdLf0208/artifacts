// Runs at the head of `Game::native_living_incoming_damage` (API 50). The hook
// is static, so the Shock Pendant's loadout comes from the cache the player
// tick refreshed.
fn cancel_lightning_damage(entity: i32, kind: &str, damage_type: &str, canceled: &mut bool) {
    crate::game::artifacts_state::on_living_incoming_damage(entity, kind, damage_type, canceled);
}
