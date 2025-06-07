use tauri::{App, Manager};
use tauri::tray::{MouseButtonState, TrayIconBuilder, TrayIconEvent};
use rosc::{OscMessage, OscPacket};
use vrchat_osc::{ServiceType, VRChatOSC};
use vrchat_osc::models::OscRootNode;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
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

async fn process_osc() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize VRChatOSC instance
    let vrchat_osc = VRChatOSC::new().await?;

    let cloned_vrchat_osc = vrchat_osc.clone();
    vrchat_osc.on_connect(move |res| match res {
        ServiceType::Osc(name, addr) => {
            println!("Connected to OSC server: {} at {}", name, addr);
            let vrchat_osc = cloned_vrchat_osc.clone();
            // Send a message to the OSC server
            tokio::spawn(async move {
                vrchat_osc.send_to_addr(
                    OscPacket::Message(OscMessage {
                        addr: "/avatar/parameters/VRChatOSC".to_string(),
                        args: vec![rosc::OscType::String("Connected".to_string())],
                    }),
                    addr,
                ).await.unwrap();
                println!("Sent message to OSC server.");
            });
        }
        ServiceType::OscQuery(name, addr) => {
            println!("Connected to OSCQuery server: {} at {}", name, addr);
            let vrchat_osc = cloned_vrchat_osc.clone();
            // Get parameters from the OSCQuery server
            tokio::spawn(async move {
                // NOTE: When actually retrieving parameters, you should implement retry logic here.
                // If VRChat has just started, it is possible that valid values may not be returned immediately.
                let params = vrchat_osc.get_parameter_from_addr("/avatar/parameters", addr).await.unwrap();
                println!("Received parameters: {:?}", params);
            });
        }
    }).await;

    // Register a test service
    let root_node = OscRootNode::new().with_avatar();
    vrchat_osc.register("test_service", root_node, |packet| {
        if let OscPacket::Message(msg) = packet {
            println!("Received OSC message: {:?}", msg);
        }
    }).await?;
    println!("Service registered.");

    // Wait for the service to be registered
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Send a test message to the registered service
    vrchat_osc.send(
        OscPacket::Message(OscMessage {
            addr: "/chatbox/input".to_string(),
            args: vec![
                rosc::OscType::String("Hello, VRChat!".to_string()),
                rosc::OscType::Bool(true),
            ],
        }),
        "VRChat-Client-*"
    ).await?;
    println!("Test message sent to VRChat-Client-*.");

    // Get parameters from the registered service
    let params = vrchat_osc.get_parameter("/avatar/parameters", "VRChat-Client-*").await?;
    println!("Received parameters: {:?}", params);
    //
    loop {
        tokio::task::yield_now().await;
    }
}

fn setup_osc_server(app: &mut App) {
    let app_handle =  app.app_handle();
    let _osc_handle = tauri::async_runtime::spawn(async move {
        process_osc()
            .await.unwrap()
    });
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