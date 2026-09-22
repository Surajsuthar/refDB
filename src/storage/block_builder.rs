use bytes::{BufMut, Bytes, BytesMut};

use crate::storage::block::{Block, SIZE_U16};

#[derive(Debug, Clone)]
pub struct BlockBuilder {
    pub data: Vec<u8>,
    pub offsets: Vec<u16>,
    pub size: usize,
}

impl BlockBuilder {
    fn create(size: usize) -> Self {
        Self {
            data: Vec::new(),
            offsets: Vec::new(),
            size,
        }
    }

    fn estimate_size(&self) -> usize {
        SIZE_U16 + SIZE_U16 * self.offsets.len() + self.data.len()
    }

    fn add(&mut self, key: &[u8], val: &[u8]) -> bool {
        assert!(!key.is_empty(), "Key Should Not be Empty");
        let key_len = key.len();
        let val_len = val.len();

        if self.estimate_size() + key_len + val_len >= self.size {
            return false;
        }

        self.offsets.push(self.data.len() as u16);
        self.data.put_u16(key_len as u16);
        self.data.put(key);
        self.data.put_u16(val_len as u16);
        self.data.put(val);

        true
    }

    fn build(&self) -> Block {
        Block {
            data: self.data.clone(),
            offsets: self.offsets.clone(),
        }
    }
}
