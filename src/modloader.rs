use mlua::prelude::{LuaResult, LuaInteger};
use thiserror::Error;
use mlua;

use crate::Mod;
use crate::factorio_concepts::LocalisedStringEntry;

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
            // TODO: locale handler
            fn localised_print(callback_lua: &mlua::Lua, data: mlua::Value) -> LuaResult<()> {
                println!("{}", callback_lua.unpack::<LocalisedStringEntry>(data)?);
                Ok(())
            }

            // Log to file
            fn lua_log(callback_lua: &mlua::Lua, data: mlua::Value) -> LuaResult<()> {
                println!("[LOG] {}", callback_lua.unpack::<LocalisedStringEntry>(data)?);
                Ok(())
            }

            // TODO: Use lua_tablesize
            fn table_size(_callback_lua: &mlua::Lua, data: mlua::Value) -> LuaResult<LuaInteger> {
                match data {
                    mlua::Value::Table(table) => table.len(),
                    _ => Err(mlua::Error::external(ModLoaderErr::InvalidType)),
                }
            }

            let globals = lua.globals();

            // I tried making helper function.
            // My brain now is melted
            globals.raw_set("localised_print", lua
                .create_function(localised_print)
                .map_err(|_| ModLoaderErr::LuaFunctionCreation)?).map_err(|_| ModLoaderErr::GlobalSetFailure)?;
            globals.raw_set("log", lua
                .create_function(lua_log)
                .map_err(|_| ModLoaderErr::LuaFunctionCreation)?).map_err(|_| ModLoaderErr::GlobalSetFailure)?;
            globals.raw_set("table_size", lua
                .create_function(table_size)
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
    #[error("Failed to create LuaFunction")]
    LuaFunctionCreation,
    #[error("Failed to set global")]
    GlobalSetFailure,
    #[error("Invalid type")]
    InvalidType,
}
