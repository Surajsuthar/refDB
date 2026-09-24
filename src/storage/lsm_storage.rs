use std::{collections::HashMap, sync::Arc};

use crate::storage::{engine::Engine, memtable::Memtable, sst};

pub struct LsmStorageEngine {
    pub memtable: Arc<Memtable>,
    pub immutable_memtable: Vec<Arc<Memtable>>,
    pub sstables: HashMap<usize, Arc<sst::SstBlock>>,
}

impl LsmStorageEngine {
    fn create() -> Self {
        Self {
            memtable: Arc::new(Memtable::new(0)),
            immutable_memtable: Vec::new(),
            sstables: Default::default(),
        }
    }
}

impl Engine for LsmStorageEngine {
    fn delete(&mut self, key: &[u8]) {
        unimplemented!();
    }

    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        unimplemented!();
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        unimplemented!();
    }
}
