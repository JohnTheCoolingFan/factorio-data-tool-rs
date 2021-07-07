use std::ffi::OsStr;
use std::fs::DirEntry;
use std::cmp::Ordering;
use lexical_sort::natural_only_alnum_cmp;
use semver::Version;
use serde::Deserialize;
use crate::ModDataErr;
use crate::dependency::{ModDependencyType, ModDependency};

// enum for states of a mod (enabled or disabled)
#[derive(Debug)]
pub enum ModEnabledType {
    Disabled,
    Latest,           // Legacy from factorio_mod_manager, probably will be renamed
    Version(Version), // Legacy from factorio_mod_manager, probably will be removed
}

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
