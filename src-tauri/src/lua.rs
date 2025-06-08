use crate::application_event::ApplicationEvent;
use mlua::prelude::LuaResult;
use mlua::Lua;
use std::sync::mpsc::{Receiver, Sender};

pub struct LuaEngine {
    lua: Lua,
    tx: Sender<ApplicationEvent>,
}

pub enum LuaEngineEvent {
    Reload,
}

impl LuaEngine {
    pub fn new(tx: Sender<ApplicationEvent>, rx: Receiver<LuaEngineEvent>) -> LuaEngine {
        LuaEngine {
            lua: Lua::new(),
            tx,
        }
    }
    pub fn main(self) -> LuaResult<()> {
        let lua = self.lua;

        // std::thread::sleep(std::time::Duration::from_secs(10));
        // self.tx.send(ApplicationEvent::Exit).unwrap();
        Ok(())
    }
}
