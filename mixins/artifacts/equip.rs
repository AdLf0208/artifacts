// Runs at the head of `Game::native_use_item` (API 13): a right click with an
// artifact moves it from the main hand into the off hand, the port's worn slot.
fn observe_equip(
    game: &mut crate::game::Game<'_>,
    stack: &horizon_client::items::ItemStack,
    claimed: &mut bool,
) {
    crate::game::artifacts_state::on_use_item(game, stack, claimed);
}
