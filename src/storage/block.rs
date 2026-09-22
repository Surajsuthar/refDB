//! Block layout used inside an SSTable.
//!
//! A block stores multiple sorted key-value entries in a compact binary format.
//! The block index keeps offsets to each entry so reads can jump directly to a
//! candidate key during binary search.
//!

use bytes::{Buf, BufMut, Bytes, BytesMut};

pub const SIZE_U16: usize = size_of::<u16>();

#[derive(Eq, PartialEq, Clone, Debug)]
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
        let buff = bytes.as_ref();
        let offset_len = (&buff[buff.len() - SIZE_U16..]).get_u16() as usize;
        let data_end = buff.len() - SIZE_U16 - (offset_len * SIZE_U16);

        let offset_space = &buff[data_end..buff.len() - SIZE_U16];

        let offsets = offset_space
            .chunks(SIZE_U16)
            .map(|mut x| x.get_u16())
            .collect();

        let data = buff[0..data_end].to_vec();

        Self { data, offsets }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]

    fn test_encode_decode() {
        let block = Block {
            data: vec![1, 2, 3],
            offsets: vec![0, 1, 2],
        };
        let encoded = block.encode();
        let decoded = Block::decode(encoded);
        assert_eq!(block, decoded);
    }
}
