pub mod media_controller;
pub mod window_manager;
pub mod audio_visualizer;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::Manager;
use tauri::image::Image;

pub fn run_gsmtc_query_cli() {
    media_controller::run_gsmtc_query_cli();
}

pub fn run_gsmtc_seek_cli(seconds: f64) {
    media_controller::run_gsmtc_seek_cli(seconds);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Desactivar el subsistema de audio de Chromium en WebView2 para aislar completamente audiodg.exe
    unsafe {
        std::env::set_var(
            "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
            "--mute-audio --disable-audio-output --disable-features=AudioServiceOutOfProcess",
        );
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // 1. Configurar menú del System Tray (Bandeja del sistema)
            let toggle_item = MenuItem::with_id(app, "toggle", "Mostrar / Ocultar", true, None::<&str>)?;
            let snap_item = MenuItem::with_id(app, "snap", "Alinear a la esquina", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Cerrar", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&toggle_item, &snap_item, &quit_item])?;
            let tray_icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;

            let _tray = TrayIconBuilder::new()
                .icon(tray_icon)
                .menu(&tray_menu)
                .tooltip("Banana Music Controller")
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "quit" => {
                        app.exit(0);
                    }
                    "toggle" => {
                        if let Some(win) = app.get_webview_window("main") {
                            if let Ok(is_visible) = win.is_visible() {
                                if is_visible {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                        }
                    }
                    "snap" => {
                        if let Some(win) = app.get_webview_window("main") {
                            let _ = window_manager::snap_to_bottom_right(win.as_ref().window());
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(win) = app.get_webview_window("main") {
                            if let Ok(is_visible) = win.is_visible() {
                                if is_visible {
                                    let _ = win.hide();
                                } else {
                                    let _ = win.show();
                                    let _ = win.set_focus();
                                }
                            }
                        }
                    }
                })
                .build(app)?;

            media_controller::start_media_sync_loop(app.handle().clone());
            audio_visualizer::start_audio_visualizer(app.handle().clone());
            Ok(())
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            "close" | "quit" => {
                app.exit(0);
            }
            "snap" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = window_manager::snap_to_bottom_right(win.as_ref().window());
                }
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            window_manager::snap_to_bottom_right,
            window_manager::close_app,
            window_manager::show_widget_context_menu,
            media_controller::get_media_state,
            media_controller::media_play_pause,
            media_controller::media_next,
            media_controller::media_previous,
            media_controller::media_seek,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
