mod lua;
mod osc;

use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{App, Manager};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_log::Builder::new().build())
        .setup(|app| {
            spawn_lua_thread(app)?;
            setup_tray_menu(app)?;
            setup_osc_server(app);
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                window.hide().unwrap();
                api.prevent_close();
            }
        })
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn spawn_lua_thread(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.app_handle();
    std::thread::spawn(move || {
        let engine = lua::LuaEngine::new();
        let _ = engine.main();
    });
    Ok(())
}

fn setup_osc_server(app: &mut App) {
    let app_handle = app.app_handle();
    let _osc_handle =
        tauri::async_runtime::spawn(async move { osc::OscService::process_osc().await.unwrap() });
}

fn setup_tray_menu(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                app.exit(0);
            }
            _ => (),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(webview_window) = app.get_webview_window("main") {
                    let _ = webview_window.show();
                    let _ = webview_window.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}
