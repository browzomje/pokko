#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if pokko::cli::is_cli_invocation(&args) {
        pokko::cli::run_cli_sync();
        return;
    }
    pokko::run_app();
}
