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
mod factorio_concepts;
mod ini_serde;
mod modloader;

use std::fs;
use std::fs::{File, DirEntry};
use std::io::Read;
use std::collections::HashMap;
use std::path::{PathBuf};
use std::error::Error;
use thiserror::Error;
use zip::ZipArchive;

use crate::dependency::{ModDependency, ModDependencyResult, ModDependencyType};
use crate::factorio_concepts::{ModVersion, ModStructure, Mod, ModListJson, ModEnabledType, InfoJson};
use crate::modloader::ModLoader;

fn main() -> Result<(), Box<dyn Error>> {
    // Mods directory path. Contains mod files/dirs, mod-list.json and mod-settings.dat
    // TODO: As a command-line argument
    let path = "mods/";

    // Read mod-list.json to know which mods are disabled
    let mut mlj_path = PathBuf::from(path);
    mlj_path.push("mod-list.json");
    let mut enabled_versions: HashMap<String, ModEnabledType> = {
        if mlj_path.exists() {
            let mlj_contents = fs::read_to_string(&mlj_path)?;
            serde_json::from_str::<ModListJson>(&mlj_contents)?
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

    // Mods HashMap, contains all mods
    let mut mods: HashMap<String, Mod> = HashMap::new();
    for entry in fs::read_dir(path)?.filter_map(|entry| {
        let entry = entry.ok()?;
        let file_name = entry.file_name();
        let file_name = file_name.to_str()?;
        if file_name != "mod-list.json" && file_name != "mod-settings.dat" {
            Some(entry)
        } else {
            None
        }
    }) {
        // Determine which structure mod is (zip file, directory, symlink)
        let mod_structure = ModStructure::parse(&entry)?;

        // Read info.json of a mod
        let info: InfoJson = match mod_structure {
            ModStructure::Zip => {
                find_info_json_in_zip(&entry)?
            }
            _ => {
                let mut path = entry.path();
                path.push("info.json");
                let contents = fs::read_to_string(path)?;
                let json: InfoJson = serde_json::from_str(&contents)?;
                json
            }
        };

        // Get the mod entry from hashmap or insert default
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

        // Construct ModVersion with dependency info
        let mod_version = ModVersion {
            entry,
            dependencies: info
                .dependencies
                .unwrap_or(vec![]) // FIXME: missing dependency list results in base dependency (default: ["base"])
                .iter()
                .map(ModDependency::new)
                .collect::<ModDependencyResult>()?,
            structure: mod_structure,
            version: info.version,
        };

        // Save latest mod version for the mod
        match &mod_data.version {
            Some(ver) if ver <= &mod_version => (),
            _ => mod_data.version = Some(mod_version),
        }
    }

    // factorio_mod_manager "partial" copy-paste ends here (for main part)

    // core will be hard-coded. AFAIK there are no mods that require core and it's invalid
    // dependency
    
    // Make mod loading list
    let mods_to_load: Vec<Mod> = {
        let (_, mut values): (Vec<String>, Vec<Mod>) = mods.drain().unzip();
        // Remove disabled mods from the list
        values.retain(|modd| match modd.enabled {
            ModEnabledType::Latest => true,
            ModEnabledType::Disabled => false,
            _ => false,
        });
        // Insert base mod. Loading of base mod is hard-coded, as it's info is included with
        // the tool
        values.push(Mod {
            name: "base".to_string(),
            version: None,
            enabled: ModEnabledType::Latest,
        });
        // Check for incompatibilities
        for modd in &values {
            match &modd.version {
                Some(version) => {
                    for dependency in &version.dependencies {
                        match dependency.dep_type {
                            ModDependencyType::Incompatible => {
                                for mod_name in values.iter().map(|modd_data| modd_data.name.clone()) {
                                    if mod_name == dependency.name{
                                        return Err(Box::new(ModDataErr::IncompatibleMods(modd.name.clone(), mod_name)));
                                    }
                                }
                            },
                            _ => (),
                        };
                    };
                },
                _ => ()
            };
        }
        values.sort_unstable();
        values.reverse();
        values
    };

    // Load mods

    let mut mod_loader = ModLoader::new(mods_to_load);
    
    // WIP
    Ok(())
}

// Find info.json file in a mod contained in zip file
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
    #[error("Incompatible mods: {0} is incompatible with {1}")]
    IncompatibleMods(String, String),
}
