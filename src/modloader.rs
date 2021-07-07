use mlua::prelude::LuaResult;
use crate::Mod;
use std::error::Error;
use thiserror::Error;
use mlua;

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

            globals.raw_set("localised_print", lua.create_function(localised_print).unwrap()).unwrap();
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
}
