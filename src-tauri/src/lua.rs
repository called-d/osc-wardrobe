use crate::application_event::ApplicationEvent;
use fs_extra;
use log::{debug, trace};
use mlua::prelude::LuaResult;
use mlua::Lua;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

pub struct LuaEngineOption {
    pub application_event_sender: Sender<ApplicationEvent>,
    pub lua_engine_event_receiver: Receiver<LuaEngineEvent>,
    pub base_dir: PathBuf,
}

pub struct LuaEngine {
    lua: Lua,
    option: LuaEngineOption,
}

pub enum LuaEngineEvent {
    Reload,
}

impl LuaEngine {
    pub fn new(option: LuaEngineOption) -> LuaEngine {
        trace!("LuaEngine::new");
        LuaEngine {
            lua: Lua::new(),
            option,
        }
    }
    pub fn main(self) -> LuaResult<()> {
        let lua = self.lua;

        let main_file = self.option.base_dir.join("main.lua");
        debug!("main.lua exists: {}", main_file.exists());
        // std::thread::sleep(std::time::Duration::from_secs(10));
        // self.tx.send(ApplicationEvent::Exit).unwrap();
        Ok(())
    }
}

pub fn extract_lua_dir_if_needed<P, Q>(
    src: P,
    lua_dir: Q,
) -> Result<bool, Box<dyn std::error::Error>>
where
    P: AsRef<Path>,
    Q: AsRef<Path>,
{
    if lua_dir.as_ref().is_dir() {
        Ok(false)
    } else {
        fs_extra::dir::copy(
            src,
            lua_dir.as_ref().parent().unwrap(),
            &fs_extra::dir::CopyOptions::new(),
        )
        .expect("再帰的なファイルコピー");
        Ok(true)
    }
}
