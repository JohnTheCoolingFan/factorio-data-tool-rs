use crate::Mod;
use std::error::Error;
use thiserror::Error;
use rlua;

#[derive(Debug)]
struct ModLoader {
    lua: rlua::Lua,
    mod_list: Vec<Mod>
}

// Do I have to make my own require()?
impl ModLoader {
    fn new(mod_list: Vec<Mod>) -> Result<Self, ModLoaderErr> {
        let lua = rlua::Lua::new();
        return Ok(Self {
            lua,
            mod_list
        })
    }

    fn load_lualib() -> Result<(), ModLoaderErr> {
        let lualib_path = "factorio-data/core/lualib";
        
    }
}

#[derive(Debug, Error)]
pub enum ModLoaderErr {
    #[error("Mod Loader Error")]
    GeneralError,
    #[error("Failed to load lualib")]
    LuaLibLoadError
}
