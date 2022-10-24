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
