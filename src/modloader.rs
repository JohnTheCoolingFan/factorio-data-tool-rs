use mlua::prelude::LuaResult;
use crate::Mod;
use std::error::Error;
use thiserror::Error;
use mlua;

// General TODO
//  - include base and core (lua files)
//  - include some locale from base and core. They are around 10MB in size, so not everything will
//  fit, en will be definitely included, and maybe will be the only available locale. And then
//  option for choosing included locale files. Total en is around 250KB
//  - Sprite data parser that will get info about sprites from existing Factorio installation. The
//  only valuable data is sprite resolution + path in the mod. This will be done for core and base
//  only by default.
//  - Mod settings

#[derive(Debug)]
pub struct ModLoader {
    lua: mlua::Lua,
    mod_list: Vec<Mod>,
    current_mod: Option<Mod>
}

// Do I have to make my own require()?
impl ModLoader {
    // TODO; https://lua-api.factorio.com/latest/Libraries.html
    // TODO: custom package.searchers function

    pub fn new(mod_list: Vec<Mod>) -> Result<Self, ModLoaderErr> {
        let lua = mlua::Lua::new();

        // Add global lua functions. For more info, visit:
        // https://lua-api.factorio.com/latest/Libraries.html
        {
            let globals = lua.globals();

            // TODO: actual functionality
            fn localised_print(_callback_lua: &mlua::Lua, data: mlua::Value) -> LuaResult<()> {
                match data {
                    mlua::Value::String(data_str) => println!("{}", data_str.to_str().unwrap()),
                    mlua::Value::Table(data_table) => println!("Localised print: {:?}", data_table),
                    _ => println!("Invalid call to localised_print")
                }
                Ok(())
            }

            globals.raw_set("localised_print", lua
                .create_function(localised_print)
                .map_err(|_| ModLoaderErr::LuaFunctionCreation)?).map_err(|_| ModLoaderErr::GlobalSetFailure)?;
        }

        return Ok(Self {
            lua,
            mod_list,
            current_mod: None
        })
    }
}

#[derive(Debug, Error)]
pub enum ModLoaderErr {
    #[error("Mod Loader Error")]
    GeneralError,
    #[error("Failed to load lualib")]
    LuaLibLoadError,
    #[error("Failed to create LuaFunction")]
    LuaFunctionCreation,
    #[error("Failed to set global")]
    GlobalSetFailure,
}
