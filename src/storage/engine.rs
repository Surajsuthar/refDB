use std::sync::{Arc, Mutex, RwLock};

use crate::storage::lsm_storage::LsmStorageEngine;

pub trait Engine {
    fn read(&self, key: &[u8]) -> Option<Vec<u8>>;
    fn write(&mut self, key: &[u8], value: &[u8]);
    fn delete(&mut self, key: &[u8]);
}

pub struct LsmTree {
    storage: RwLock<Arc<LsmStorageEngine>>,
    lock: Mutex<()>,
}
