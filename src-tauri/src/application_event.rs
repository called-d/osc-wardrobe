pub enum ApplicationEvent {
    SendOsc(String, serde_json::Value),
    ReloadLua,
    Exit,
}
