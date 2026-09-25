use std::{collections::HashMap, sync::Arc};

use crate::storage::{engine::LsmEngine, memtable::Memtable, sst};

pub struct LsmStorageEngineState {
    pub memtable: Arc<Memtable>,
    pub immutable_memtable: Vec<Arc<Memtable>>,
    pub sstables: HashMap<usize, Arc<sst::SstBlock>>,
}

impl LsmStorageEngineState {
    fn create() -> Self {
        Self {
            memtable: Arc::new(Memtable::new(0)),
            immutable_memtable: Vec::new(),
            sstables: Default::default(),
        }
    }
}

impl LsmEngine {
    fn delete(&mut self, key: &[u8]) {
        unimplemented!();
    }

    fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        unimplemented!();
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        unimplemented!();
    }
}
