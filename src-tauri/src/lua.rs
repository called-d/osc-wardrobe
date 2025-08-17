use crate::application_event::ApplicationEvent;
use fs_extra;
use log::{debug, trace, warn};
use mlua::prelude::{LuaMultiValue, LuaResult};
use mlua::{Function, IntoLuaMulti, Lua, LuaSerdeExt, MultiValue, Table};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};

pub struct LuaEngineOption {
    pub application_event_sender: Sender<ApplicationEvent>,
    pub lua_engine_event_receiver: Receiver<LuaEngineEvent>,
    pub base_dir: PathBuf,
}

pub struct LuaEngine {
    lua: std::sync::Mutex<Lua>,
    option: LuaEngineOption,
}

pub enum LuaEngineEvent {
    OscReceived(String, serde_json::Value),
    DefinitionUpdated(serde_json::Value),
    Reload,
}

impl LuaEngine {
    pub fn new(option: LuaEngineOption) -> LuaEngine {
        trace!("LuaEngine::new");
        let engine = LuaEngine {
            lua: std::sync::Mutex::new(Lua::new()),
            option,
        };
        engine.load_libraries();
        engine
    }
    async fn reload(&mut self) -> LuaResult<()> {
        *self.lua.get_mut().expect("get mut") = Lua::new();
        self.load_libraries();
        self.main().await
    }
    pub async fn main(&self) -> LuaResult<()> {
        let lua = &self.lua.lock().expect("get lock for main()");

        let main_path = self.option.base_dir.join("main.lua");
        debug!("main.lua exists: {}", main_path.exists());
        let mut main = std::fs::File::open(&main_path)?;
        let mut buffer = String::new();
        main.read_to_string(&mut buffer)?;
        let _ = lua
            .load(buffer.as_str())
            .exec_async()
            .await
            .expect("load main file");
        drop(main);
        if let Ok(main) = lua.globals().get::<mlua::Function>("main") {
            let return_value = main.call_async::<MultiValue>(()).await?;
            debug!("main returns {:?}", return_value);
        };
        Ok(())
    }

    pub async fn process_event(&mut self) -> () {
        // trace!("LuaEngine::process_event");
        loop {
            if let Ok(event) = self.option.lua_engine_event_receiver.try_recv() {
                match event {
                    LuaEngineEvent::OscReceived(s, v) => {
                        let args = {
                            self.lua
                                .lock()
                                .expect("get lock for receive()")
                                .to_value(&v)
                        };
                        if let Err(e) = self.call_function("receive", (s, args)).await {
                            warn!("error on  Osc receive event: {:?}", e);
                        };
                    }
                    LuaEngineEvent::DefinitionUpdated(v) => {
                        debug!("Definition updated event: {:?}", v);
                        if let Err(e) = self.set_global(&["wardrobe", "definition"], v) {
                            warn!("error on  DefinitionUpdated event: {:?}", e);
                        };
                    }
                    LuaEngineEvent::Reload => self.reload().await.expect("reload"),
                }
            } else {
                break;
            }
        }
    }

    async fn call_function(
        &self,
        function_name: &str,
        args: impl IntoLuaMulti,
    ) -> mlua::Result<()> {
        let f = {
            let lua = &self.lua.lock().expect("get lock for call_function()");
            lua.globals().get::<Function>(function_name)?
        };
        f.call_async(args).await?;
        Ok(())
    }

    fn set_global(&self, keys: &[&str], value: serde_json::Value) -> mlua::Result<()> {
        let (last_key, keys) = keys.split_last().expect("keys must be non-empty");
        let lua = &self.lua.lock().expect("get lock for set_global()");
        let mut table = lua.globals();
        for key in keys {
            if let Ok(mlua::Value::Table(t)) = table.get(key.to_string()) {
                table = t
            } else {
                let t = lua.create_table()?;
                table.set(key.to_string(), &t)?;
                table = t
            }
        }
        table
            .set(
                last_key.to_string(),
                lua.to_value(&value).expect("to_value"),
            )
            .expect("set_global key=value");
        Ok(())
    }

    fn load_libraries(&self) {
        let lua = &self.lua.lock().expect("get lock for load_libraries()");
        let package_loaded = lua
            .named_registry_value::<Table>("_LOADED")
            .expect("_LOADED");

        /* ### osc library ### */
        let osc_lib = lua.create_table().expect("create_table osc_lib");
        let sender = self.option.application_event_sender.clone();
        osc_lib
            .set(
                "send",
                lua.create_function(move |lua, args: LuaMultiValue| {
                    if args.len() < 2 {
                        return Ok([
                            lua.null(),
                            mlua::Value::String(lua.create_string("no address and args").unwrap()),
                        ]);
                    }
                    let Some(mlua::Value::String(addr)) = args.get(0) else {
                        return Ok([
                            lua.null(),
                            mlua::Value::String(
                                lua.create_string("address is not string").unwrap(),
                            ),
                        ]);
                    };
                    sender
                        .send(ApplicationEvent::SendOsc(
                            addr.to_string_lossy(),
                            serde_json::to_value(&args.into_vec()[1..]).unwrap(),
                        ))
                        .expect("application event send osc");
                    Ok([mlua::Value::Boolean(true), lua.null()])
                })
                .expect("create_function"),
            )
            .expect("osc.send =");
        package_loaded.set("osc", &osc_lib).expect("osc");
        lua.globals().set("osc", osc_lib).expect("osc");

        /* ### wardrobe app bridge ### */
        let wardrobe_lib = lua.create_table().expect("create_table wardrobe_lib");
        let sender = self.option.application_event_sender.clone();
        wardrobe_lib
            .set(
                "exit",
                lua.create_function(move |_, _: LuaMultiValue| {
                    sender
                        .send(ApplicationEvent::Exit)
                        .expect("application event exit");
                    Ok([mlua::Value::Nil])
                })
                .expect("create_function"),
            )
            .expect("wardrobe.exit =");
        package_loaded
            .set("wardrobe", &wardrobe_lib)
            .expect("wardrobe");
        lua.globals()
            .set("wardrobe", wardrobe_lib)
            .expect("wardrobe");

        /* ### sleep ### */
        let sleep = lua
            .create_async_function(move |_lua, s: f32| async move {
                debug!("sleep({:?})", s);
                tokio::time::sleep(tokio::time::Duration::from_secs_f32(s)).await;
                Ok(())
            })
            .expect("create_function");
        lua.globals().set("sleep", sleep).expect("sleep");
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
