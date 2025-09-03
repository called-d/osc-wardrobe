use crate::AppState;
use regex::Regex;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::LazyLock;
use std::sync::{Arc, Mutex};
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase", tag = "event", content = "data")]
pub enum LogEvent {
    #[serde(rename_all = "camelCase")]
    Log { line: String },
    #[serde(rename_all = "camelCase")]
    Print { line: String },
    #[serde(rename_all = "camelCase")]
    Finished,
}

pub struct LogState {
    pub sender: std::sync::mpsc::Sender<String>,
    map: HashMap<String, Channel<LogEvent>>,
}

#[tauri::command]
pub fn get_logs(
    state: tauri::State<Mutex<AppState>>,
    target: String,
    log_event: Channel<LogEvent>,
) {
    {
        let state = state.lock().expect("get state (get_logs)");
        let mut log_state = state.log_state.lock().expect("get log_state");
        log_state.map.insert(target, log_event);
    }
}

fn get_target_name(str: &String) -> Option<&str> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^\[\d{4}-\d{2}-\d{2}]\[\d{2}:\d{2}:\d{2}]\[(?P<module>[\w:]+?)]").unwrap()
    });
    let Some(captures) = RE.captures(&str) else {
        return None;
    };

    match &captures["module"] {
        "osc_wardrobe_lib::lua" => Some("lua"),
        "osc_wardrobe_lib::osc" => Some("osc"),
        _ => None,
    }
}

impl LogState {
    pub fn create() -> (LogState, std::sync::mpsc::Receiver<String>) {
        let (tx, rx) = channel();
        (
            LogState {
                sender: tx,
                map: HashMap::new(),
            },
            rx,
        )
    }

    pub fn print_to_log(&self, line: &str) {
        if let Some(channel) = self.map.get("lua") {
            channel
                .send(LogEvent::Print {
                    line: line.to_string(),
                })
                .unwrap();
        }
    }

    pub async fn process(state: Arc<Mutex<LogState>>, receiver: std::sync::mpsc::Receiver<String>) {
        loop {
            tokio::task::yield_now().await;

            if let Ok(str) = receiver.recv() {
                if let Some(target) = get_target_name(&str) {
                    if let Some(channel) = state.lock().expect("process.state").map.get(target) {
                        channel.send(LogEvent::Log { line: str.clone() }).unwrap();
                    }
                }
                if let Some(channel) = state.lock().expect("process.state").map.get("all") {
                    channel.send(LogEvent::Log { line: str }).unwrap();
                }
            }
        }
    }
}
