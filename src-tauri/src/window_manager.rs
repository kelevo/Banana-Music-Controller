use tauri::{AppHandle, PhysicalPosition, Window};
use tauri::menu::{ContextMenu, Menu, MenuItem};

#[cfg(target_os = "windows")]
#[repr(C)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[cfg(target_os = "windows")]
#[repr(C)]
struct MonitorInfo {
    cb_size: u32,
    rc_monitor: Rect,
    rc_work: Rect,
    dw_flags: u32,
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn MonitorFromWindow(hwnd: isize, dw_flags: u32) -> isize;
    fn GetMonitorInfoW(h_monitor: isize, lpmi: *mut MonitorInfo) -> i32;
}

/// Menú contextual nativo de Windows (Clic derecho sobre el widget)
#[tauri::command]
pub fn show_widget_context_menu(window: Window) -> Result<(), String> {
    let menu = Menu::new(&window).map_err(|e| e.to_string())?;
    let snap_item = MenuItem::with_id(&window, "snap", "Alinear a la esquina", true, None::<&str>)
        .map_err(|e| e.to_string())?;
    let close_item = MenuItem::with_id(&window, "close", "Cerrar", true, None::<&str>)
        .map_err(|e| e.to_string())?;

    menu.append(&snap_item).map_err(|e| e.to_string())?;
    menu.append(&close_item).map_err(|e| e.to_string())?;
    menu.popup(window).map_err(|e| e.to_string())?;
    Ok(())
}

/// Posiciona la ventana en la esquina inferior derecha del monitor actual
/// respetando el área de trabajo (barra de tareas) y un margen de 16px.
#[tauri::command]
pub fn snap_to_bottom_right(window: Window) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "No se encontró el monitor activo actual".to_string())?;

    let scale_factor = monitor.scale_factor();
    let margin = (16.0 * scale_factor).round() as i32;
    let window_size = window.outer_size().map_err(|e| e.to_string())?;
    let window_w = window_size.width as i32;
    let window_h = window_size.height as i32;

    #[cfg(target_os = "windows")]
    {
        if let Ok(hwnd) = window.hwnd() {
            let hwnd_val = hwnd.0 as isize;
            const MONITOR_DEFAULTTONEAREST: u32 = 2;
            let hmonitor = unsafe { MonitorFromWindow(hwnd_val, MONITOR_DEFAULTTONEAREST) };
            if hmonitor != 0 {
                let mut mi = MonitorInfo {
                    cb_size: std::mem::size_of::<MonitorInfo>() as u32,
                    rc_monitor: Rect { left: 0, top: 0, right: 0, bottom: 0 },
                    rc_work: Rect { left: 0, top: 0, right: 0, bottom: 0 },
                    dw_flags: 0,
                };
                let success = unsafe { GetMonitorInfoW(hmonitor, &mut mi) };
                if success != 0 {
                    let x = mi.rc_work.right - window_w - margin;
                    let y = mi.rc_work.bottom - window_h - margin;

                    window
                        .set_position(tauri::Position::Physical(PhysicalPosition { x, y }))
                        .map_err(|e| e.to_string())?;
                    return Ok(());
                }
            }
        }
    }

    // Fallback general basado en las dimensiones reportadas por Tauri
    let monitor_pos = monitor.position();
    let monitor_size = monitor.size();
    let x = monitor_pos.x + monitor_size.width as i32 - window_w - margin;
    let y = monitor_pos.y + monitor_size.height as i32 - window_h - margin;

    window
        .set_position(tauri::Position::Physical(PhysicalPosition { x, y }))
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// Cierre limpio y ordenado de la aplicación
#[tauri::command]
pub fn close_app(app_handle: AppHandle) {
    app_handle.exit(0);
}
