// this file will be memtable implenation

use anyhow::Result;
use bytes::Bytes;
use crossbeam_skiplist::SkipMap;
use std::sync::{Arc, atomic::AtomicUsize};

pub struct Memtable {
    map: Arc<SkipMap<Bytes, Bytes>>,
    memtable_size: AtomicUsize,
    mem_id: usize,
}

impl Memtable {
    pub fn new(mem_id: usize) -> Self {
        Self {
            map: Arc::new(SkipMap::new()),
            memtable_size: AtomicUsize::new(0),
            mem_id,
        }
    }

    pub fn put(&self, key: &[u8], val: &[u8]) -> Result<()> {
        assert!(!key.is_empty(), "Key should not be empty");

        let size = key.len() + val.len();
        self.map
            .insert(Bytes::copy_from_slice(key), Bytes::copy_from_slice(val));

        self.memtable_size
            .fetch_add(size, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }

    // can be value and tomb
    pub fn get(&self, key: &[u8]) -> Option<Bytes> {
        assert!(!key.is_empty(), "Key should not be empty");
        self.map.get(key).map(|val| val.value().clone())
    }

    pub fn memtable_size(&self) -> usize {
        self.memtable_size
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn scan(&self, start: &[u8], end: &[u8]) -> Vec<(Bytes, Bytes)> {
        unimplemented!()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    #[test]
    fn memtable_insert() {
        let mem = Memtable::new(0);
        mem.put(b"key1", b"val1").unwrap();
        mem.put(b"key2", b"val2").unwrap();
        mem.put(b"key3", b"val3").unwrap();

        let val1 = &mem.get(b"key1").unwrap()[..];
        assert_eq!(val1, b"val1");

        let val2 = &mem.get(b"key2").unwrap()[..];
        assert_eq!(val2, b"val2");
    }

    #[test]
    fn memtable_empty() {
        let mem = Memtable::new(0);
        assert_eq!(true, mem.is_empty());
        mem.put(b"key1", b"val1").unwrap();
        assert_eq!(false, mem.is_empty());
    }

    #[test]
    fn memtable_overwrite_test() {
        let mem = Memtable::new(0);
        mem.put(b"key1", b"val1").unwrap();
        mem.put(b"key1", b"val2").unwrap();
        mem.put(b"key1", b"val3").unwrap();

        let val1 = &mem.get(b"key1").unwrap()[..];
        assert_eq!(val1, b"val3");
    }

    #[test]
    fn memtable_tombstone() {
        let mem = Memtable::new(0);
        mem.put(b"key1", b"").unwrap();

        let val1 = &mem.get(b"key1").unwrap()[..];
        assert_eq!(true, val1.is_empty());
    }

    #[test]
    fn concurrent_insert_to_mem() {
        use std::sync::Arc;
        use std::thread;

        let mem = Arc::new(Memtable::new(0));
        let mut handles = Vec::new();

        for thread_id in 0..10 {
            let mem = Arc::clone(&mem);

            let handle = thread::spawn(move || {
                for i in 0..1000 {
                    let key = format!("key-{thread_id}-{i}");
                    let val = format!("value-{thread_id}-{i}");

                    mem.put(key.as_bytes(), val.as_bytes()).unwrap();
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(mem.map.len(), 10_000);
    }

    #[test]
    fn concurrent_read_write() {
        let mem = Arc::new(Memtable::new(0));
        let mut handler = Vec::new();
        for num_thread in 0..4 {
            let mem = Arc::clone(&mem);

            handler.push(thread::spawn(move || {
                for i in 0..1000 {
                    let key = format!("key-{num_thread}-{i}");
                    let val = format!("value-{num_thread}-{i}");

                    mem.put(key.as_bytes(), val.as_bytes()).unwrap();
                }
            }));
        }

        for thread_id in 0..4 {
            let mem = Arc::clone(&mem);

            handler.push(thread::spawn(move || {
                for i in 0..1000 {
                    let key = format!("key-{thread_id}-{i}");

                    // It is okay if the key isn't there yet.
                    let _ = mem.get(key.as_bytes());
                }
            }));
        }

        for handle in handler {
            handle.join().unwrap();
        }
    }
}
