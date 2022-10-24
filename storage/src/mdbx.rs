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

use std::{path::Path, sync::Arc};

use libmdbx::{DatabaseFlags, Environment, NoWriteMap, WriteFlags};
use serde::{Deserialize, Serialize};

use crate::{Storage, StorageData};

#[derive(Debug, Clone)]
pub struct MdbxStorage {
    env: Arc<Environment<NoWriteMap>>,
}

impl Storage for MdbxStorage {
    fn get<T: for<'a> Deserialize<'a> + StorageData>(&self, key: impl AsRef<[u8]>) -> Option<T> {
        let txn = self.env.begin_ro_txn().unwrap();
        if let Ok(db) = txn.open_db(Some(&T::name())) {
            if let Ok(Some(v)) = txn.get::<Vec<u8>>(&db, key.as_ref()) {
                let value = bincode::deserialize::<T>(&v);
                if let Ok(value) = value {
                    return Some(value);
                }
            }
        }
        None
    }

    fn insert<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>, value: T) -> Option<T> {
        let txn = self.env.begin_rw_txn().unwrap();
        let db = match txn.open_db(Some(&T::name())) {
            Ok(db) => db,
            Err(_) => txn
                .create_db(Some(&T::name()), DatabaseFlags::empty())
                .unwrap(),
        };
        if txn
            .put(
                &db,
                key,
                bincode::serialize(&value).unwrap(),
                WriteFlags::empty(),
            )
            .is_ok()
        {
            txn.commit().unwrap();
            return Some(value);
        }
        None
    }

    fn remove<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>) -> bool {
        let txn = self.env.begin_rw_txn().unwrap();
        if let Ok(db) = txn.open_db(Some(&T::name())) {
            return txn.del(&db, key, None).is_ok();
        }
        false
    }

    /// get next value from a named sequence
    fn next(&self, name: &str) -> u32 {
        const TREE_NAME: &str = "sequence";
        let txn = self.env.begin_rw_txn().unwrap();
        let db = match txn.open_db(Some(TREE_NAME)) {
            Ok(db) => db,
            Err(_) => txn
                .create_db(Some(TREE_NAME), DatabaseFlags::empty())
                .unwrap(),
        };
        let mut result = 0;
        if let Ok(r) = txn.get::<Vec<u8>>(&db, name.as_bytes()) {
            if let Some(v) = r {
                let next = bincode::deserialize::<u32>(&v);
                if let Ok(next) = next {
                    if let Ok(()) = txn.put(
                        &db,
                        name,
                        bincode::serialize(&(next + 1)).unwrap(),
                        WriteFlags::empty(),
                    ) {
                        result = next + 1;
                    } else {
                        result = 0;
                    };
                }
            } else if let Ok(()) = txn.put(
                &db,
                name,
                bincode::serialize(&1).unwrap(),
                WriteFlags::empty(),
            ) {
                result = 1;
            } else {
                result = 0;
            }
        }
        txn.commit().unwrap();
        result
    }

    /// get current value from a named sequence
    fn current(&self, name: &str) -> u32 {
        const TREE_NAME: &str = "sequence";
        let txn = self.env.begin_ro_txn().unwrap();
        let db = match txn.open_db(Some(TREE_NAME)) {
            Ok(db) => db,
            Err(_) => return 0,
        };
        if let Ok(Some(v)) = txn.get::<Vec<u8>>(&db, name.as_bytes()) {
            let next = bincode::deserialize::<u32>(&v);
            if let Ok(next) = next {
                return next;
            }
        }
        0
    }
}

impl MdbxStorage {
    pub fn init(path: impl AsRef<Path>) -> Self {
        Self {
            env: Arc::new(
                Environment::new()
                    .set_max_dbs(8)
                    .open(path.as_ref())
                    .unwrap(),
            ),
        }
    }
}

#[test]
pub fn sequence() {
    let store = MdbxStorage::init("../test_db");
    println!("{}", store.current("test"));
    for _ in 0..100 {
        println!("{}", store.next("test"));
    }
}
