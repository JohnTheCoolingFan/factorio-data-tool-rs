use zip::ZipArchive;
use std::fs::File;
use std::ops::Index;
use std::io::Read;
use std::collections::HashMap;
use mlua::prelude::LuaResult;
use std::ffi::OsStr;
use std::fs::DirEntry;
use std::cmp::Ordering;
use std::fmt;
use lexical_sort::natural_only_alnum_cmp;
use semver::Version;
use serde::Deserialize;
use thiserror::Error;
use ini::Ini;

use crate::ModDataErr;
use crate::dependency::{ModDependencyType, ModDependency};

// enum for states of a mod (enabled or disabled)
#[derive(Debug)]
pub enum ModEnabledType {
    Disabled,
    Latest,           // Legacy from factorio_mod_manager, probably will be renamed
    Version(Version), // Legacy from factorio_mod_manager, probably will be removed
}

// Structs and enums for representing mod info related data

// Mod struct, containing mod name, version and enabled info
#[derive(Debug)]
pub struct Mod {
    pub name: String,
    pub version: Option<ModVersion>,
    pub enabled: ModEnabledType,
}

// impls for sorting the mod list for loading order
impl PartialOrd for Mod {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.name == other.name {
            return None;
        }
        Some(self.cmp(other))
    }
}

impl Ord for Mod {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.has_dependency(&other.name) {
            return Ordering::Greater;
        } else {
            return natural_only_alnum_cmp(&self.name, &other.name);
        }
    }
}

impl PartialEq for Mod {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version && self.name == other.name
    }
}

impl Eq for Mod {}

impl Mod {
    // Check if this mod has other mod as a dependency
    fn has_dependency(&self, dep_name: &String) -> bool {
        match &self.version {
            Some(version) => {
                for dependency in &version.dependencies {
                    if &dependency.name == dep_name {
                        match &dependency.dep_type {
                            ModDependencyType::Optional | ModDependencyType::Required | ModDependencyType::OptionalHidden => true,
                            _ => false,
                        };
                    }
                }
                false
            },
            _ => false,
        }
    }
}

// Struct for Mod version (or file, terminology isn't perfect)
#[derive(Debug)]
pub struct ModVersion {
    pub entry: DirEntry,
    pub dependencies: Vec<ModDependency>,
    pub structure: ModStructure,
    pub version: Version,
}

// impls for comparing mod versions
impl PartialOrd for ModVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.version.partial_cmp(&other.version) {
            Some(Ordering::Equal) => {
                match (&self.structure, &other.structure) {
                    (ModStructure::Zip, ModStructure::Directory) | (ModStructure::Zip, ModStructure::Symlink) => Some(Ordering::Less),
                    _ => Some(Ordering::Equal),
                }
            }
            Some(ord) => Some(ord),
            _ => None,
        }
    }
}

impl Ord for ModVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.version.cmp(&other.version) {
            Ordering::Equal => {
                match (&self.structure, &other.structure) {
                    (ModStructure::Zip, ModStructure::Directory) | (ModStructure::Zip, ModStructure::Symlink) => Ordering::Less,
                    _ => Ordering::Equal,
                }
            }
            Ordering::Greater => Ordering::Greater,
            Ordering::Less => Ordering::Less,
        }
    }
}

impl PartialEq for ModVersion {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl Eq for ModVersion {}

impl ModVersion {
    pub fn find_file(&self, filename: String) -> Result<Box<dyn mlua::AsChunk>, ModDataErr> {
        match self.structure {
            ModStructure::Directory | ModStructure::Symlink => {
                let file_path = self.entry.path().join(filename);
                if file_path.exists() {
                    let file = File::create(file_path).unwrap();
                    return Ok(Box::new(file.bytes().map(|byte| byte.unwrap()).collect::<Vec<u8>>()))
                }
                else {
                    return Err(ModDataErr::FileNotFound(file_path))
                }
            },
            ModStructure::Zip => {
                let entry_path = self.entry.path();
                let file_path = entry_path.join(filename);
                let mut zip_archive = ZipArchive::new(File::create(entry_path).unwrap()) .unwrap();
                let zip_file = zip_archive.by_name(file_path.to_str().unwrap())
                    .map_err(|_| ModDataErr::FileNotFound(file_path))?;
                let bytes = zip_file.bytes().map(|byte| byte.unwrap()).collect::<Vec<u8>>();
                Ok(Box::new(bytes))
            }
        }
    }
}

#[derive(Debug)]
pub enum ModStructure {
    Directory,
    Symlink,
    Zip,
}

impl ModStructure {
    pub fn parse(entry: &DirEntry) -> Result<Self, ModDataErr> {
        let path = entry.path();
        let extension = path.extension();

        if extension.is_some() && extension.unwrap() == OsStr::new("zip") {
            return Ok(ModStructure::Zip);
        } else {
            let file_type = entry.file_type().map_err(|_| ModDataErr::FilesystemError)?;
            if file_type.is_symlink() {
                return Ok(ModStructure::Symlink);
            } else {
                let mut path = entry.path();
                path.push("info.json");
                if path.exists() {
                    return Ok(ModStructure::Directory);
                }
            }
        }

        Err(ModDataErr::InvalidModStructure)
    }
}

// Structs for deserializing json files
#[derive(Deserialize, Debug)]
pub struct InfoJson {
    pub dependencies: Option<Vec<String>>,
    pub name: String,
    pub version: Version,
}

#[derive(Deserialize)]
pub struct ModListJson {
    pub mods: Vec<ModListJsonMod>,
}

#[derive(Deserialize)]
pub struct ModListJsonMod {
    pub name: String,
    pub enabled: bool,
}

// Unfinished
#[derive(Debug)]
pub struct LocaleHandler {
    entries: HashMap<String, String>
}

impl Index<String> for LocaleHandler {
    type Output = String;

    fn index(&self, key: String) -> &Self::Output {
        self.entries.get(&key).unwrap() // Improve to not use unwrap
    }
}

impl LocaleHandler {
    pub fn new() -> Self {
        Self{entries: HashMap::new()}
    }

    pub fn append_from_reader<R: Read>(&mut self, reader: &mut R) -> Result<(), ini::Error> {
        let ini = Ini::read_from_noescape(reader)?;
        if !ini.is_empty() {
            for (section, property) in ini.iter() {
                if let Some(section) = section {
                    for (key, value) in property.iter() {
                        self.entries.insert(format!("{}.{}", section, key), value.to_string());
                    }
                }
            }
        }
        Ok(())
    }
}

// Factorio concepts
// https://lua-api.factorio.com/latest/Concepts.html

// LocalisedString
#[derive(Debug)]
pub struct LocalisedString<'a> {
    pub value: mlua::Value<'a>,
    locale_entries: HashMap<String, &'a str>,
}

impl fmt::Display for LocalisedString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // To print actual localised strings, access to locale info is needed, located in .cfg (ini) files
        match &self.value {
            mlua::Value::String(value_str) => write!(f, "{}", value_str.to_str().unwrap()), // There should be a better way, without unwrap
            mlua::Value::Number(value_num) => write!(f, "{}", value_num),
            mlua::Value::Integer(value_int) => write!(f, "{}", value_int),
            mlua::Value::Table(value_table) => write!(f, "table loc str {:?}", value_table), // TODO: Actual behaviour
            _ => write!(f, "Wrong value type") // TODO: Actual error
        }
    }
}

impl<'lua> mlua::FromLua<'lua> for LocalisedString<'lua> {
    fn from_lua(value: mlua::Value<'lua>, _: &'lua mlua::Lua) -> LuaResult<Self> {
        match value {
            mlua::Value::String(_) => Ok(Self{value, locale_entries: HashMap::new()}),
            mlua::Value::Table(_) => Ok(Self{value, locale_entries: HashMap::new()}),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "LocalisedString",
                message: None,
            }),
        }
    }
}

impl LocalisedString<'_> {
    // TODO
    fn fill_lookup(&mut self, locale_handler: &LocaleHandler) {
    }
}

// Error enum for concepts
#[derive(Debug, Error)]
pub enum ConceptsErr<'a> {
    #[error("Invalid LocalisedString value type: {0:?}")]
    InvalidLocalisedStringType(&'a mlua::Value<'a>) // Was used in LocalisedString::new, which caused headaches in development
}
