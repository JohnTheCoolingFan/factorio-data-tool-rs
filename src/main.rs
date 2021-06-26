/* Factorio data tester
 * Purpose of this program is to test the mods.
 * It loads mods.
 *
 * Basic working algorithm:
 * 1. List all files in mods/, except mod-settings.dat and mmod-list.json
 * 2. Check which of them are mods
 * 3. Select latest versions of mods, priorities unpacked versions (dirs)
 * 4. Read mod-list.json and disable mods that are disabled (low-priority task)
 * 5. Create dependency tree.
 * 6. Create Lua instance using rlua patched to use Factorio lua
 * 7. Load lualib from core
 * 8. Load settings.lua
 * 9. Parse mod-settings.dat, if present (low priority task)
 * 10. Then goes basic data lifecycle https://lua-api.factorio.com/latest/Data-Lifecycle.html
 * 11. in the end, iterate over each loaded prototype and check if all the data is correct
 *      This includes checking sprite sizes, missing or redundant entries , etc.
 */

mod dependency;

use std::cmp::Ordering;
use zip::ZipArchive;
use std::fs::File;
use std::ffi::OsStr;
use std::collections::HashMap;
use std::path::{PathBuf};
use std::fs::DirEntry;
use std::fs;
use std::io::Read;
use std::error::Error;
use thiserror::Error;
use serde::Deserialize;
use semver::Version;
use lexical_sort::natural_only_alnum_cmp;

use crate::dependency::{ModDependency, ModDependencyResult, ModDependencyType};

fn main() {
    let path = "mods/";

    let mut mlj_path = PathBuf::from(path);
    mlj_path.push("mod-list.json");
    let mut enabled_versions: HashMap<String, ModEnabledType> = {
        if mlj_path.exists() {
            let mlj_contents = fs::read_to_string(&mlj_path).unwrap();
            serde_json::from_str::<ModListJson>(&mlj_contents).unwrap()
                .mods
                .iter()
                .filter_map(|entry| {
                    Some((
                        entry.name.clone(),
                        match entry.enabled {
                            true => ModEnabledType::Latest,
                            _ => ModEnabledType::Disabled,
                        },
                    ))
                })
                .collect()
        } else {
            HashMap::new()
        }
    };

    let mut mods: HashMap<String, Mod> = HashMap::new();
    for entry in fs::read_dir(path).unwrap().filter_map(|entry| {
        let entry = entry.unwrap();
        let file_name = entry.file_name();
        let file_name = file_name.to_str().unwrap();
        if file_name != "mod-list.json" && file_name != "mod-settings.dat" {
            Some(entry)
        } else {
            None
        }
    }) {
        let mod_structure = ModStructure::parse(&entry).unwrap();

        let info: InfoJson = match mod_structure {
            ModStructure::Zip => {
                find_info_json_in_zip(&entry).unwrap()
            }
            _ => {
                let mut path = entry.path();
                path.push("info.json");
                let contents = fs::read_to_string(path).unwrap();
                let json: InfoJson = serde_json::from_str(&contents).unwrap();
                json
            }
        };

        let mod_data = mods.entry(info.name.clone()).or_insert(Mod {
            name: info.name.clone(),
            version: None,
            enabled: {
                let active_version = enabled_versions.remove(&info.name);
                match active_version {
                    Some(enabled_type) => enabled_type,
                    None => ModEnabledType::Latest,
                }
            }
        });

        let mod_version = ModVersion {
            entry,
            dependencies: info
                .dependencies
                .unwrap_or(vec![]) // FIXME: exmpty dependency list results in base dependency (default: ["base"])
                .iter()
                .map(ModDependency::new)
                .collect::<ModDependencyResult>().unwrap(),
            structure: mod_structure,
            version: info.version,
        };

        match &mod_data.version {
            Some(ver) if ver <= &mod_version => (),
            _ => mod_data.version = Some(mod_version),
        }
    }

    // factorio_mod_manager "partial" copy-paste ends here (for main part)

    // core will be hard-coded. AFAIK there are no mods that require core and it's invalid
    // dependency
    let mods_to_load: Vec<Mod> = {
        let (_, mut values): (Vec<String>, Vec<Mod>) = mods.drain().unzip();
        values.push(Mod {
            name: "base".to_string(),
            version: None,
            enabled: ModEnabledType::Latest,
        });
        values.retain(|modd| match modd.enabled {
            ModEnabledType::Latest => true,
            ModEnabledType::Disabled => false,
            _ => false,
        });
        values.sort_unstable();
        values
    };
    // WIP
}

fn find_info_json_in_zip(entry: &DirEntry) -> Result<InfoJson, Box<dyn Error>> {
    let file = File::open(entry.path())?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        if file.name().contains("info.json") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            // serde_json::from_reader could be used
            let json: InfoJson = serde_json::from_str(&contents)?;
            return Ok(json);
        }
    }
    Err("Mod ZIP does not containt an info.json file".into())
}

#[derive(Debug, Error)]
pub enum ModDataErr {
    #[error("Filesystem error")]
    FilesystemError,
    #[error("Invalid mod sctucture")]
    InvalidModStructure,
    #[error("Mod does not exist")]
    ModDoesNotExist,
}

#[derive(Debug)]
pub enum ModEnabledType {
    Disabled,
    Latest,
    Version(Version),
}

#[derive(Debug)]
struct Mod {
    name: String,
    version: Option<ModVersion>,
    enabled: ModEnabledType,
}

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

#[derive(Debug)]
struct ModVersion {
    entry: DirEntry,
    dependencies: Vec<ModDependency>,
    structure: ModStructure,
    version: Version,
}

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
enum ModStructure {
    Directory,
    Symlink,
    Zip,
}

impl ModStructure {
    fn parse(entry: &DirEntry) -> Result<Self, ModDataErr> {
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

#[derive(Deserialize, Debug)]
struct InfoJson {
    dependencies: Option<Vec<String>>,
    name: String,
    version: Version,
}

#[derive(Deserialize)]
struct ModListJson {
    mods: Vec<ModListJsonMod>,
}

#[derive(Deserialize)]
struct ModListJsonMod {
    name: String,
    enabled: bool,
}
