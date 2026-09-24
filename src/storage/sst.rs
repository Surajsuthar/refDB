use std::{
    fs::File,
    os::unix::fs::{FileExt, MetadataExt},
    path::Path,
};

use bytes::Buf;

use crate::error::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockMeta {
    first_key: Vec<u8>,
    last_key: Vec<u8>,
    offset: usize,
}

impl BlockMeta {
    fn encode_block_meta(meta: &[BlockMeta], buff: &mut Vec<u8>) {
        let mut total_size = 0usize;
        for meta in meta {
            total_size += std::mem::size_of::<u32>(); // offset
            total_size += std::mem::size_of::<u16>();
            total_size += meta.first_key.len();
            total_size += std::mem::size_of::<u16>();
            total_size += meta.last_key.len();
        }

        buff.reserve(total_size);
    }

    fn decode_block_meta(mut buf: impl Buf) -> BlockMeta {
        unimplemented!()
    }
}

//file object with filesize
pub struct FileObject(File, u64);

impl FileObject {
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::options()
            .read(true)
            .write(false)
            .open(path)
            .expect("error while opening file at path: {path}");
        let size = file.metadata().expect("error while opening file").size();
        Ok(FileObject(file, size))
    }
    pub fn read(&self, offset: u64, len: u32) -> Result<Vec<u8>> {
        let mut buff = vec![0u8; len as usize];
        self.0.read_exact_at(&mut buff[..], offset);
        Ok(buff)
    }

    pub fn create(path: &Path, data: Vec<u8>) -> Result<()> {
        unimplemented!();
    }
}

pub struct SstBlock {
    pub file: FileObject,
    pub block_meta: Option<BlockMeta>,
    pub block_meta_offser: usize,
}
