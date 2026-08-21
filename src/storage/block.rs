//! Block layout used inside an SSTable.
//!
//! A block stores multiple sorted key-value entries in a compact binary format.
//! The block index keeps offsets to each entry so reads can jump directly to a
//! candidate key during binary search.
//!

use bytes::{BufMut, Bytes, BytesMut};

pub const SIZE_U16: usize = size_of::<u16>();

pub(super) struct Block {
    pub data: Vec<u8>,
    pub offsets: Vec<u16>,
}

impl Block {
    fn encode(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(self.size());

        buf.put_slice(&self.data);
        for offset in &self.offsets {
            buf.put_u16(*offset);
        }
        buf.put_u16(self.offsets.len() as u16);
        buf.freeze()
    }

    fn size(&self) -> usize {
        SIZE_U16 + self.offsets.len() * SIZE_U16 + self.data.len()
    }

    fn decode(bytes: Bytes) -> Self {
        let bytes = bytes.as_ref();
        unimplemented!()
    }
}
