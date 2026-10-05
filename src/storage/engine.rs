use std::sync::{Arc, Mutex, RwLock};

use crate::{
    error::Result,
    storage::{compact::CompactionOptions, lsm_storage::LsmStorageEngineState},
};

pub trait Engine {
    fn begin(&mut self) -> Result<()>;
    fn commit(&mut self) -> Result<()>;
    fn rollback(&mut self) -> Result<()>;

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
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
