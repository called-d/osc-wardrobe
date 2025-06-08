mod application_event;
mod lua;
mod osc;

use crate::application_event::ApplicationEvent;
use crate::lua::LuaEngineEvent;
use log::*;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, SubmenuBuilder};
use tauri::path::BaseDirectory;
use tauri::tray::{TrayIconBuilder, TrayIconEvent};
use tauri::{App, AppHandle, Manager};
use tauri_plugin_cli::CliExt;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_cli::init())
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(LevelFilter::Trace)
                .level_for("vrchat_osc::mdns::task", LevelFilter::Info)
                .level_for("hickory_proto::rr::record_data", LevelFilter::Info)
                .level_for("vrchat_osc::mdns::utils", LevelFilter::Info)
                .level_for("vrchat_osc::mdns::task", LevelFilter::Warn)
                .build(),
        )
        .setup(|app| {
            let (tx, rx) = channel();
            let lua_engine_event_sender = setup_lua(app, tx.clone())?;
            setup_tray_menu(app, tx.clone())?;
            let osc_receiver = setup_osc_server(app);
            setup_event_processor(app, rx, osc_receiver, lua_engine_event_sender);
            info!("setup done.");
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

fn setup_lua(
    app: &App,
    tx: Sender<ApplicationEvent>,
) -> Result<Sender<LuaEngineEvent>, Box<dyn std::error::Error>> {
    debug!("extract lua directory");
    let lua_dir_src = app
        .path()
        .resolve("resources/lua", BaseDirectory::Resource)?;

    let lua_dir = lua_dir(app.app_handle());

    if let Ok(matches) = app.cli().matches() {
        if let Some(arg_data) = matches.args.get("overwrite-all-lua") {
            if let serde_json::Value::Bool(true) = arg_data.value {
                debug!("overwrite-all-lua true");
                fs_extra::dir::remove(&lua_dir)?;
            }
        }
    }
    match app.cli().matches() {
        Ok(matches) => {
            println!(
                "overwrite all lua, {:?}",
                matches.args.get("overwrite-all-lua")
            )
        }
        Err(_) => {}
    }

    if lua::extract_lua_dir_if_needed(lua_dir_src, &lua_dir)? {
        debug!("extracted: {:?}", lua_dir);
    } else {
        debug!("already exists");
    }

    debug!("spawn lua_thread");
    let (tx2, rx2) = channel();
    std::thread::spawn(move || {
        let engine = lua::LuaEngine::new(lua::LuaEngineOption {
            base_dir: lua_dir,
            lua_engine_event_receiver: rx2,
            application_event_sender: tx,
        });
        let _ = engine.main();
    });
    Ok(tx2)
}

fn setup_osc_server(app: &mut App) -> tokio::sync::mpsc::UnboundedReceiver<osc::OscEvent> {
    let app_handle = app.app_handle();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    debug!("setup_osc_server: start");
    let _osc_handle = tauri::async_runtime::spawn(async move {
        osc::OscService::process_osc(tx).await.unwrap();
    });
    rx
}
fn setup_event_processor(
    app: &mut App,
    application_event_receiver: Receiver<ApplicationEvent>,
    mut osc_receiver: tokio::sync::mpsc::UnboundedReceiver<osc::OscEvent>,
    lua_sender: Sender<LuaEngineEvent>,
) {
    let app_handle = app.app_handle().clone();
    let _ = tauri::async_runtime::spawn(async move {
        loop {
            tokio::task::yield_now().await;
            if let Ok(app_event) = application_event_receiver.try_recv() {
                match app_event {
                    ApplicationEvent::Exit => {
                        app_handle.exit(0);
                    }
                }
            }
            tokio::select! {
                Some(osc_msg) = osc_receiver.recv() => { match osc_msg {
                    osc::OscEvent::Message(message) => {
                        debug!("osc received {:?}", message);
                    }
                } },
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(10)) => {},
                else => {
                    warn!("channel is closed");
                },
            }
        }
    });
}

fn lua_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .resolve("lua", BaseDirectory::AppData)
        .expect("lua dir not found")
}
fn open_dir<P>(path: P) -> Result<(), Box<dyn std::error::Error>>
where
    P: AsRef<std::path::Path>,
{
    tauri_plugin_opener::open_path(path, None::<&str>)?;
    Ok(())
}

fn setup_tray_menu(
    app: &mut App,
    tx: Sender<ApplicationEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    let directory_menu = SubmenuBuilder::new(app, "Open Directory")
        .text("directory_lua", "Lua")
        .text("directory_common", "Common")
        .build()?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&directory_menu, &separator, &quit_i])?;

    let sender_ = tx.clone();
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => sender_.send(ApplicationEvent::Exit).unwrap(),
            "directory_lua" => open_dir(lua_dir(app)).unwrap(),
            _ => (),
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::DoubleClick { .. } = event {
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
