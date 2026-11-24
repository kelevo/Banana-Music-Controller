// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.iter().any(|arg| arg == "--query-gsmtc") {
        banana_music_controller_lib::run_gsmtc_query_cli();
        return;
    }

    if let Some(pos) = args.iter().position(|arg| arg == "--seek-gsmtc") {
        if let Some(val_str) = args.get(pos + 1) {
            if let Ok(sec) = val_str.parse::<f64>() {
                banana_music_controller_lib::run_gsmtc_seek_cli(sec);
                return;
            }
        }
    }

    banana_music_controller_lib::run()
}
