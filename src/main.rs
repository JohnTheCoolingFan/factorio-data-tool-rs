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

fn main() {
    println!("Hello, world!");
}
