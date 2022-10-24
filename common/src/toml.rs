use std::{fs, path::Path};

use serde::Deserialize;
use toml::Value;

pub fn read_toml<'a, T: Deserialize<'a>>(path: impl AsRef<Path>) -> T {
    let s = fs::read_to_string(path)
        .map_err(|e| println!("read_to_string err: {}", e))
        .unwrap();
    let config: Value = s
        .parse()
        .map_err(|e| println!("toml parse err: {}", e))
        .unwrap();
    T::deserialize(config)
        .map_err(|e| println!("config deserialize err: {}", e))
        .unwrap()
}
