mod application_event;
mod lua;
mod osc;

use crate::application_event::ApplicationEvent;
use crate::lua::LuaEngineEvent;
use log::*;
use notify_debouncer_mini::{new_debouncer, notify::RecursiveMode};
use std::path::{Path, PathBuf};
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
            let (tx2, rx2) = tokio::sync::mpsc::channel(1000);
            let lua_engine_event_sender = setup_lua(app, tx.clone())?;
            setup_definitions(app, lua_engine_event_sender.clone())?;
            setup_tray_menu(app, tx.clone())?;
            let osc_receiver = setup_osc_server(app, rx2);
            setup_event_processor(app, rx, osc_receiver, lua_engine_event_sender, tx2);
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
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let mut engine = lua::LuaEngine::new(lua::LuaEngineOption {
                base_dir: lua_dir,
                lua_engine_event_receiver: rx2,
                application_event_sender: tx,
            });
            let _ = engine.main().await;

            loop {
                engine.process_event().await;
            }
        });
    });
    Ok(tx2)
}

fn get_definition(defs_dir: &PathBuf) -> serde_json::Value {
    trace!("get definition: {:?}", defs_dir);
    if !defs_dir.exists() {
        return serde_json::Value::Null;
    }
    trace!("get definition _");
    let mut table = serde_json::json!({});
    for entry in walkdir::WalkDir::new(defs_dir)
        .follow_links(true)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
    {
        if !is_json(&entry) {
            continue;
        }
        let path = entry.path();
        let Some(keys) = get_keys(defs_dir, path) else {
            continue;
        };
        // read json from file
        let Ok(file) = std::fs::File::open(path) else {
            warn!("could not open definition file: {:?}", path);
            continue;
        };
        let Ok::<serde_json::value::Value, _>(json) = serde_json::from_reader(file) else {
            warn!("could not parse definition file: {:?}", path);
            continue;
        };
        set_value(&mut table, &keys, json);
    }
    table
}
fn is_hidden(entry: &walkdir::DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| entry.depth() > 0 && s.starts_with("."))
        .unwrap_or(false)
}
fn is_json(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_file() {
        return false;
    }
    entry.path().extension() == Some("json".as_ref())
}
fn get_keys(root: &PathBuf, path: &Path) -> Option<Vec<String>> {
    let Some(stem) = path.file_stem() else {
        return None;
    };
    let path = path.with_file_name(stem);
    let Ok(sub) = path.strip_prefix(root) else {
        return None;
    };
    Some(
        sub.components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
    )
}
fn set_value(table: &mut serde_json::value::Value, keys: &[String], value: serde_json::Value) {
    let Some((first_key, keys)) = keys.split_first() else {
        return;
    };
    let Some(map) = table.as_object_mut() else {
        return;
    };
    if keys.len() == 0 {
        map.insert(first_key.into(), value);
        return;
    }
    if let Some(maybe_table) = map.get_mut(first_key) {
        if maybe_table.is_object() {
            set_value(maybe_table, keys, value);
            return;
        }
    }
    map.insert(first_key.into(), serde_json::json!({}));
    set_value(map.get_mut(first_key).unwrap(), keys, value);
}

fn setup_definitions(
    app: &App,
    lua_event_sender: Sender<LuaEngineEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    trace!("setup definitions");
    let app_handle = app.app_handle().clone();
    let defs_dir = defs_dir(&app_handle);
    if !defs_dir.exists() {
        std::fs::create_dir_all(&defs_dir)?;
    }
    lua_event_sender
        .send(LuaEngineEvent::DefinitionUpdated(get_definition(&defs_dir)))
        .unwrap();

    let (tx, rx) = channel();
    let mut debouncer = new_debouncer(std::time::Duration::from_secs(2), tx)?;
    let _ = tauri::async_runtime::spawn(async move {
        let watcher = debouncer.watcher();
        watcher
            .watch(&defs_dir, RecursiveMode::Recursive)
            .expect("watcher start");
        loop {
            tokio::task::yield_now().await;
            if let Ok(event) = rx.try_recv() {
                match event {
                    Ok(event) => {
                        debug!("event: {:?}", event);
                        lua_event_sender
                            .send(LuaEngineEvent::DefinitionUpdated(get_definition(&defs_dir)))
                            .unwrap();
                    }
                    Err(e) => {
                        warn!("notify error: {:?}", e);
                    }
                }
            } else {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    });
    Ok(())
}

fn setup_osc_server(
    app: &mut App,
    receiver: tokio::sync::mpsc::Receiver<osc::OscEvent>,
) -> tokio::sync::mpsc::UnboundedReceiver<osc::OscEvent> {
    let app_handle = app.app_handle();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    debug!("setup_osc_server: start");
    let _osc_handle = tauri::async_runtime::spawn(async move {
        osc::OscService::process_osc(tx, receiver).await.unwrap();
    });
    rx
}
fn osc_to_json(v: &rosc::OscType) -> serde_json::Value {
    use rosc::OscType::*;
    use serde_json::json;
    match v {
        Int(i) => json!(i),
        Float(f) => json!(f),
        String(s) => json!(s),
        Long(l) => json!(l.to_string()),
        Double(d) => json!(d),
        Char(c) => json!(c),
        Color(color) => json!((color.red, color.green, color.blue, color.alpha)),
        Bool(b) => json!(b),
        Array(arr) => arr.content.iter().map(osc_to_json).collect(),
        Nil => json!(null),
        Inf => json!(f32::INFINITY),
        _ => {
            debug!("not implemented for {:?}", v);
            json!(null)
        }
    }
}
fn json_to_osc(v: &serde_json::Value) -> rosc::OscType {
    use serde_json::Value::*;
    match v {
        Bool(b) => rosc::OscType::Bool(*b),
        Null => rosc::OscType::Nil,
        Number(n) => rosc::OscType::Float(
            serde_json::from_str::<f32>(&serde_json::to_string(n).unwrap()).unwrap(),
        ),
        String(s) => rosc::OscType::String(s.to_string()),
        Array(a) => rosc::OscType::Array(a.iter().map(json_to_osc).collect()),
        _ => {
            debug!("not implemented for {:?}", v);
            rosc::OscType::Nil
        }
    }
}
fn setup_event_processor(
    app: &mut App,
    application_event_receiver: Receiver<ApplicationEvent>,
    mut osc_receiver: tokio::sync::mpsc::UnboundedReceiver<osc::OscEvent>,
    lua_sender: Sender<LuaEngineEvent>,
    osc_sender: tokio::sync::mpsc::Sender<osc::OscEvent>,
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
                    ApplicationEvent::SendOsc(addr, args) => {
                        osc_sender
                            .send(osc::OscEvent::Message(rosc::OscMessage {
                                addr,
                                args: args
                                    .as_array()
                                    .expect("args is array")
                                    .iter()
                                    .map(json_to_osc)
                                    .collect(),
                            }))
                            .await
                            .unwrap_or_else(|err| {
                                warn!("failed to send OSC message (mpsc event queue): {:?}", err);
                            });
                    }
                    ApplicationEvent::ReloadLua => lua_sender
                        .send(LuaEngineEvent::Reload)
                        .expect("failed to send LuaEngineEvent::Reload"),
                }
            }
            tokio::select! {
                Some(osc_msg) = osc_receiver.recv() => { match osc_msg {
                    osc::OscEvent::Message(message) => {
                        debug!("osc received {:?}", message);
                        lua_sender.send(LuaEngineEvent::OscReceived(
                            message.addr,
                            serde_json::Value::Array(message.args.iter().map(osc_to_json).collect()),
                        )).unwrap();
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
        .expect("lua dir resolve")
}
fn lua_io_dir(app: &AppHandle) -> PathBuf {
    lua_dir(&app).join("io")
}
fn defs_dir(app: &AppHandle) -> PathBuf {
    app.path()
        .resolve("defs", BaseDirectory::AppData)
        .expect("defs dir resolve")
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
    let lua_menu = SubmenuBuilder::new(app, "Lua")
        .text("lua_reload", "Reload")
        .separator()
        .build()?;
    let directory_menu = SubmenuBuilder::new(app, "Open Folder")
        .text("directory_lua", "Lua")
        .text("directory_defs", "Definitions")
        .text("directory_io", "I/O")
        .build()?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&lua_menu, &directory_menu, &separator, &quit_i])?;

    let sender_ = tx.clone();
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "quit" => sender_.send(ApplicationEvent::Exit).unwrap(),
            "directory_lua" => open_dir(lua_dir(app)).unwrap(),
            "directory_io" => {
                let io_dir = lua_io_dir(app);
                if !io_dir.exists() {
                    std::fs::create_dir_all(&io_dir).unwrap();
                }
                open_dir(io_dir).unwrap()
            }
            "directory_defs" => open_dir(defs_dir(app)).unwrap(),
            "lua_reload" => {
                debug!("reload");
                sender_.send(ApplicationEvent::ReloadLua).unwrap()
            }
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
