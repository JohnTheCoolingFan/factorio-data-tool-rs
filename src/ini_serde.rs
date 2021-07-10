use serde::{Serialize, Deserialize};
use std::collections::HashMap;

// I'm not using serde-ini (https://github.com/arcnmx/serde-ini) because:
// 1. It's not perfect
// 2. My use case is rather specialized
// 3. I want to experiment

pub struct LocaleFile {
    entries: HashMap<String, String> // Map "category.name" to actual locale string. There is no need to get category with all its entries
}
