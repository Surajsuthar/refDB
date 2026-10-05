use std::{
    fs::File,
    io::BufWriter,
    path::Path,
    sync::{Arc, Mutex},
};

use crate::error::Result;

pub struct Wal {
    file: Arc<Mutex<BufWriter<File>>>,
}

impl Wal {
    pub fn create(_path: impl AsRef<Path>) -> Result<Self> {
        unimplemented!();
    }
    pub fn sync(&self) -> Result<()> {
        unimplemented!();
    }
    pub fn write(&self, key: &[u8], value: &[u8]) -> Result<()> {
        unimplemented!();
    }
    pub fn write_batch(&self, batch: &[(&[u8], &[u8])]) {
        unimplemented!();
    }
}
