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
    pub fn new(mod_list: Vec<Mod>) -> Result<Self, ModLoaderErr> {
        let lua = mlua::Lua::new();
        return Ok(Self {
            lua,
            mod_list,
            current_mod: None
        })
    }
    
    // TODO; https://lua-api.factorio.com/latest/Libraries.html
    // TODO: custom package.searchers function
}

#[derive(Debug, Error)]
pub enum ModLoaderErr {
    #[error("Mod Loader Error")]
    GeneralError,
    #[error("Failed to load lualib")]
    LuaLibLoadError
}
