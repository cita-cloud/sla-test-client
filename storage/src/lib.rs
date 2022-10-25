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

pub mod sledb;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait StorageData: Debug + Clone + Default + for<'a> Deserialize<'a> + Serialize {
    fn name() -> String;
}

pub trait Storage: Debug + Clone {
    fn get<T: for<'a> Deserialize<'a> + StorageData>(&self, key: impl AsRef<[u8]>) -> Option<T>;
    fn all<T: for<'a> Deserialize<'a> + StorageData>(&self) -> Vec<T>;
    fn insert<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>, value: T) -> Option<T>;
    fn remove<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>) -> bool;
    fn next(&self, name: &str) -> u32;
    fn current(&self, name: &str) -> u32;
}
