pub mod mdbx;
pub mod sledb;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

pub trait StorageData: Debug + Clone + Default + for<'a> Deserialize<'a> + Serialize {
    fn name() -> String;
}

pub trait Storage: Debug + Clone {
    fn get<T: for<'a> Deserialize<'a> + StorageData>(&self, key: impl AsRef<[u8]>) -> Option<T>;
    fn insert<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>, value: T) -> Option<T>;
    fn remove<T: Serialize + StorageData>(&self, key: impl AsRef<[u8]>) -> bool;
    fn next(&self, name: &str) -> u32;
    fn current(&self, name: &str) -> u32;
}
