// Copyright Rivtower Technologies LLC.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::{fs, path::Path};

use serde::Deserialize;
use toml::Value;

pub fn read_toml<'a, T: Deserialize<'a>>(path: impl AsRef<Path>) -> Option<T> {
    let s = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            println!("read_to_string err: {e}");
            return None;
        }
    };
    let config: Value = match s.parse() {
        Ok(config) => config,
        Err(e) => {
            println!("read_to_string err: {e}");
            return None;
        }
    };
    T::deserialize(config)
        .map_err(|e| println!("config deserialize err: {e}"))
        .ok()
}

pub fn calculate_md5(path: impl AsRef<Path>) -> Result<[u8; 16], std::io::Error> {
    match fs::read_to_string(path) {
        Ok(s) => Ok(*md5::compute(s)),
        Err(e) => Err(e),
    }
}
