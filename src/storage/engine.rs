use std::sync::{Arc, Mutex, RwLock};

use crate::{
    error::Result,
    storage::{compact::CompactionOptions, lsm_storage::LsmStorageEngineState},
};

pub trait Engine {
    fn read(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn write(&mut self, key: &[u8], value: &[u8]);
    fn delete(&mut self, key: &[u8]);
}

pub struct LsmStorageOptions {
    pub memtable_size: usize,
    pub compaction_type: CompactionOptions,
    pub block_size: usize,
}

pub struct LsmEngine {
    storage: RwLock<Arc<LsmStorageEngineState>>,
    lock: Mutex<()>,
}
