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

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{Storage, StorageData};

#[derive(Debug, Clone)]
pub struct SledStorage {
    sledb: sled::Db,
}

impl Storage for SledStorage {
    fn get<T: for<'a> Deserialize<'a> + StorageData>(&self, key: impl AsRef<[u8]>) -> Option<T> {
        let tree = self.sledb.open_tree(&T::name()).unwrap();
        if let Ok(Some(v)) = tree.get(key.as_ref()) {
            let value = bincode::deserialize::<T>(&v);
            if let Ok(value) = value {
                return Some(value);
            }
        }
        None
    }

    fn insert<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>, value: T) -> Option<T> {
        let tree = self.sledb.open_tree(&T::name()).unwrap();
        if tree
            .insert(key, bincode::serialize(&value).unwrap())
            .is_ok()
        {
            return Some(value);
        }
        None
    }

    fn remove<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>) -> bool {
        let tree = self.sledb.open_tree(&T::name()).unwrap();
        tree.remove(key).is_ok()
    }

    /// get next value from a named sequence
    fn next(&self, name: &str) -> u32 {
        const TREE_NAME: &str = "sequence";
        if let Ok(r) = self.sledb.open_tree(TREE_NAME).unwrap().get(name) {
            if let Some(v) = r {
                let next = bincode::deserialize::<u32>(&v);
                if let Ok(next) = next {
                    return if let Ok(()) = self
                        .sledb
                        .open_tree(TREE_NAME)
                        .unwrap()
                        .compare_and_swap(
                            name,
                            Some(bincode::serialize(&next).unwrap()),
                            Some(bincode::serialize(&(next + 1)).unwrap()),
                        )
                        .unwrap()
                    {
                        next + 1
                    } else {
                        0
                    };
                }
            } else if let Ok(()) = self
                .sledb
                .open_tree(TREE_NAME)
                .unwrap()
                .compare_and_swap(
                    name,
                    None as Option<&[u8]>,
                    Some(bincode::serialize(&1).unwrap()),
                )
                .unwrap()
            {
                return 1;
            } else {
                return 0;
            }
        }
        0
    }

    /// get current value from a named sequence
    fn current(&self, name: &str) -> u32 {
        const TREE_NAME: &str = "sequence";
        if let Ok(Some(v)) = self.sledb.open_tree(TREE_NAME).unwrap().get(name) {
            let next = bincode::deserialize::<u32>(&v);
            if let Ok(next) = next {
                return next;
            }
        }
        0
    }
}

impl SledStorage {
    pub fn init(path: impl AsRef<Path>) -> Self {
        Self {
            sledb: sled::open(path).unwrap(),
        }
    }
}

#[test]
pub fn sequence() {
    let store = SledStorage::init("../test_db");
    println!("{}", store.current("test"));
    for _ in 0..100 {
        println!("{}", store.next("test"));
    }
}
