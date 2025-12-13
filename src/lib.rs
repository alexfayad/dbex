// src/lib.rs
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write, BufWriter};
use std::path::{Path, PathBuf};
use bincode::{Encode, Decode};

pub struct DBex {
    index: BTreeMap<Vec<u8>, ValueLocation>,
    file: BufWriter<File>,
    path_buf: PathBuf,
    write_pos: u64,
}

#[derive(Encode, Decode, Debug)]
struct ValueLocation {
    offset: u64,
    len: u32,
}

impl DBex {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {

        let path_buf = path.as_ref().to_path_buf();

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .append(true)
            .open(path)
            .expect("Failed to open database file");

        let write_pos =
            file.metadata().map(|m| m.len()).unwrap_or(0);
        let index =
            Self::load_index(path_buf.as_path()).unwrap_or(BTreeMap::new());

        DBex {
            index,
            file: BufWriter::with_capacity(16 * 1024 * 1024, file),  // 64MB buffer
            path_buf,
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

        let value_offset = loc.offset + 4 + key.len() as u64 + 4;

        let file = self.file.get_mut();
        file.seek(SeekFrom::Start(value_offset)).unwrap();

        let mut buf = vec![0u8; loc.len as usize];
        file.read_exact(&mut buf).unwrap();

        Some(buf)
    }

    pub fn flush(&mut self) {
        self.file.flush().unwrap();
        self.file.get_ref().sync_all().unwrap();
        self.save_index().unwrap();
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn start_txn(&mut self) {
        unimplemented!("This function is not yet implemented.")
    }

    pub fn commit_txn(&mut self) {
        // Write to WAL or some shit
        // then flush or some shit
        unimplemented!("This function is not yet implemented.")
    }

    fn save_index(&self) -> io::Result<()> {
        let bytes = bincode::encode_to_vec(&self.index, bincode::config::standard())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let index_path = self.path_buf.with_extension("db.index");
        fs::write(index_path, bytes)?;
        Ok(())
    }

    fn load_index(path: &Path) -> io::Result<BTreeMap<Vec<u8>, ValueLocation>> {
        let bytes = fs::read(path.with_extension("db.index"))?;
        let (index, _len): (BTreeMap<Vec<u8>, ValueLocation>, usize) =
            bincode::decode_from_slice(&bytes, bincode::config::standard())
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(index)
    }
}