pub mod block;
pub mod block_builder;
pub mod compact;
pub mod engine;
pub mod lsm_storage;
pub mod memtable;
pub mod sst;

pub struct LsmOptions {
    pub block_size: usize,
    pub memtable_size: usize,
    pub compaction_options: compact::CompactionOptions,
}
