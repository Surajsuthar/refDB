use std::ops::Bound;

use crate::error::Result;

pub mod engine;

pub trait Storage {
    type Txn: StorageTxn;
    fn begin(&self, read_only: bool) -> Result<Self::Txn>;
}

pub trait StorageTxn {
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    fn put(&mut self, key: &[u8], val: &[u8]) -> Result<()>;
    fn delete(&mut self, key: &[u8]) -> Result<()>;
    fn scan(
        &self,
        lo: Bound<&[u8]>,
        hi: Bound<&[u8]>,
    ) -> Result<Box<dyn Iterator<Item = Result<(Vec<u8>, Vec<u8>)>> + '_>>;
    fn commit(self) -> Result<()>;
    fn rollback(self) -> Result<()>;
}
