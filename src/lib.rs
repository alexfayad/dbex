// src/lib.rs
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write, BufWriter};
use std::path::Path;

pub struct DBex {
    index: HashMap<Vec<u8>, ValueLocation>,
    file: BufWriter<File>,
    write_pos: u64,
}

struct ValueLocation {
    offset: u64,
    len: u32,
}

impl DBex {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)
            .expect("Failed to open database file");

        let write_pos = file.metadata().map(|m| m.len()).unwrap_or(0);

        DBex {
            index: HashMap::new(),
            file: BufWriter::new(file),
            write_pos,
        }
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8]) {
        let offset = self.write_pos;

        // Format: [key_len: 4 bytes][key][value_len: 4 bytes][value]
        let key_len = key.len() as u32;
        let value_len = value.len() as u32;

        self.file.write_all(&key_len.to_be_bytes()).unwrap();
        self.file.write_all(key).unwrap();
        self.file.write_all(&value_len.to_be_bytes()).unwrap();
        self.file.write_all(value).unwrap();

        let entry_size = 4 + key.len() as u64 + 4 + value.len() as u64;
        self.write_pos += entry_size;

        self.index.insert(key.to_vec(), ValueLocation {
            offset,
            len: value_len,
        });
    }

    pub fn find(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let loc = self.index.get(key)?;

        // Calculate where value starts
        let value_offset = loc.offset + 4 + key.len() as u64 + 4;

        // Need raw file access for seeking
        let file = self.file.get_mut();
        file.seek(SeekFrom::Start(value_offset)).unwrap();

        let mut buf = vec![0u8; loc.len as usize];
        file.read_exact(&mut buf).unwrap();

        Some(buf)
    }

    pub fn flush(&mut self) {
        self.file.flush().unwrap();
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }
}