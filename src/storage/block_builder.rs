use bytes::Bytes;

use crate::storage::block::Block;

pub struct BlockBuilder {
    data: Vec<u8>,
    offsets: Vec<u64>,
    size: usize,
    first_key: Bytes,
}

impl BlockBuilder {
    fn create(size: usize) -> Self {
        Self {
            data: Vec::new(),
            offsets: Vec::new(),
            size,
            first_key: Bytes::new(),
        }
    }

    fn add(&mut self, key: &[u8], val: &[u8]) -> bool {
        unimplemented!()
    }

    fn build(&mut self) -> Block {
        unimplemented!()
    }
}
