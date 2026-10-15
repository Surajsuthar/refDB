use std::{
    collections::HashMap,
    sync::{Arc, atomic::AtomicUsize},
};

use crate::{
    error::Result,
    storage::{engine::LsmEngine, memtable::Memtable, sst, wal::Wal},
};

pub struct LsmStorageEngineState {
    pub memtable: Arc<Memtable>,
    pub immutable_memtable: Vec<Arc<Memtable>>,
    pub sstables: HashMap<usize, Arc<sst::SstBlock>>,
    next_sstable_id: AtomicUsize,
    pub l0_sstables: HashMap<usize, Arc<sst::SstBlock>>,
    pub wal: Option<Arc<Wal>>,
}

impl LsmStorageEngineState {
    fn create() -> Self {
        Self {
            memtable: Arc::new(Memtable::new(0)),
            immutable_memtable: Vec::new(),
            sstables: Default::default(),
            next_sstable_id: AtomicUsize::new(0),
            l0_sstables: Default::default(),
            wal: None,
        }
    }

    fn get_next_sstable_id(&mut self) -> usize {
        self.next_sstable_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }
}

impl LsmEngine {
    fn delete(&mut self, key: &[u8]) {
        unimplemented!();
    }

    fn read(&self, key: &[u8]) -> Option<Vec<u8>> {
        unimplemented!();
    }

    fn write(&mut self, key: &[u8], value: &[u8]) {
        unimplemented!();
    }

    fn flush_memtable(&mut self) -> Result<()> {
        unimplemented!();
    }
}
