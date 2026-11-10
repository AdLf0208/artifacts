// Before the server starts: load or create `config/artifacts.conf`.
fn load_config(game: &mut crate::game::Game<'_>) {
    crate::game::artifacts_state::on_server_about_to_start(game);
}
