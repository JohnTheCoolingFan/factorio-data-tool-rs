use crate::Mod;
use std::error::Error;
use thiserror::Error;
use hlua;

#[derive(Debug)]
struct ModLoader<'a> {
    lua: hlua::Lua<'a>,
    mod_list: Vec<Mod>
}

// Do I have to make my own require()?
impl ModLoader<'_> {
    fn new(mod_list: Vec<Mod>) -> Result<Self, ModLoaderErr> {
        let lua = hlua::Lua::new();
        return Ok(Self {
            lua,
            mod_list
        })
    }
    
    // TODO: custom package.searchers function

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
