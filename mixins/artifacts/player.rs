// End of a living player's server tick (API 31): scan the carried artifacts,
// apply their attribute modifiers and cache the loadout for the damage hook.
fn observe_player_tick(game: &mut crate::game::Game<'_>) {
    crate::game::artifacts_state::on_player_tick(game);
}
