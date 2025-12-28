use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::{ SystemTime, UNIX_EPOCH };
use crate::memtable::MemTable;

pub struct SSTable {
    data_path: PathBuf,
    data_file: BufReader<File>,
    index_path: PathBuf,
    index_file: BufReader<File>,
    sparse_index: Vec<(Vec<u8>, u64)>,
    min_key: Vec<u8>,
    max_key: Vec<u8>,
    ss_tables: Vec<SSTable>,
}

impl SSTable {
    pub fn new(memtable: &MemTable) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        let data_path = PathBuf::from(format!("ss_table_{}.db", timestamp));
        let index_path = PathBuf::from(format!("ss_table_{}.db.index", timestamp));

        let data_file = File::create(&data_path).unwrap();
        let index_file = File::create(&index_path).unwrap();

        let mut data_bufwriter = BufWriter::new(data_file);
        let mut index_bufwriter = BufWriter::new(index_file);

        let mut offset = 0u64;
        let mut index_vec = Vec::new();

        let mut sparse_index = Vec::new();
        let mut sparse_offset = 0u64;

        for (i, (key, value)) in memtable.data().iter().enumerate() {
            // Save index entry (key → current offset)
            index_vec.push((key.clone(), offset));
            offset += Self::write_entry(&mut data_bufwriter, value.clone());

            if i % 100 == 0 {
                sparse_index.push((key.clone(), sparse_offset));
            }

            let key_len = key.len() as u32;
            let entry_size = 4 + key_len as u64 + 8;
            sparse_offset += entry_size;
        }

        let (min_key, max_key) = SSTable::write_index(&mut index_bufwriter, &index_vec);

        let data_file = data_bufwriter.into_inner().unwrap();
        let index_file = index_bufwriter.into_inner().unwrap();

        data_file.sync_all().unwrap();
        index_file.sync_all().unwrap();

        // After writing, keep file in open mode versus create mode.
        let data_file = File::open(&data_path).unwrap();
        let data_file = BufReader::new(data_file);
        let index_file = File::open(&index_path).unwrap();
        let index_file = BufReader::new(index_file);

        SSTable {
            data_path,
            data_file,
            index_path,
            index_file,
            sparse_index,
            min_key,
            max_key,
            ss_tables: Vec::new(),
        }
    }

    pub fn data_path (&self) -> &PathBuf {
        &self.data_path
    }

    pub fn index_path (&self) -> &PathBuf {
        &self.index_path
    }

    pub fn min_key (&self) -> &Vec<u8> {
        &self.min_key
    }

    pub fn max_key (&self) -> &Vec<u8> {
        &self.max_key
    }

    pub fn get(&mut self, key: &Vec<u8>) -> Option<Vec<u8>> {
        if self.ss_tables.is_empty() {
            // Binary search the sparse index (O(log n) instead of O(n))
            let search_result = self.sparse_index.binary_search_by(|(k, _)| {
                k.as_slice().cmp(key)
            });

            let start_offset = match search_result {
                Ok(idx) => {
                    // Exact match in sparse index
                    let start = self.sparse_index[idx].1;
                    start
                }
                Err(idx) => {
                    // Key would be inserted at idx
                    // So it's between sparse_index[idx-1] and sparse_index[idx]
                    let start = if idx == 0 {
                        0
                    } else {
                        self.sparse_index[idx - 1].1
                    };
                    start
                }
            };

            return self.get_from_index_file(key, start_offset);
        }

        // Parent node: check children with range filtering
        for child in &mut self.ss_tables {
            if key >= child.min_key() && key <= child.max_key() {
                if let Some(value) = child.get(key) {
                    return Some(value);
                }
            }
        }

        None
    }

    pub fn get_from_index_file(&mut self, key: &[u8], offset: u64) -> Option<Vec<u8>> {
        self.index_file.seek(SeekFrom::Start(offset)).unwrap();

        loop {
            // Read key length (4 bytes)
            let mut key_len_bytes = [0u8; 4];
            if self.index_file.read_exact(&mut key_len_bytes).is_err() {
                break;
            }
            let key_len = u32::from_be_bytes(key_len_bytes) as usize;

            // Read key
            let mut stored_key = vec![0u8; key_len];
            self.index_file.read_exact(&mut stored_key).ok()?;

            let mut offset_bytes = [0u8; 8];
            self.index_file.read_exact(&mut offset_bytes).ok()?;
            let offset = u64::from_be_bytes(offset_bytes);

            if &stored_key == key {
                // Read offset (8 bytes)
                return self.read_value_at_offset(offset);
            }

            if stored_key.as_slice() > key {
                return None;
            }
        }
        None
    }

    fn read_value_at_offset(&mut self, offset: u64) -> Option<Vec<u8>> {

        self.data_file.seek(SeekFrom::Start(offset)).unwrap();

        // Read value length
        let mut len_bytes = [0u8; 4];
        self.data_file.read_exact(&mut len_bytes).unwrap();
        let value_len = u32::from_be_bytes(len_bytes) as usize;

        if value_len == 0xFFFFFFFF {
            return None;  // This key was deleted
        }

        // Read value
        let mut value = vec![0u8; value_len];
        self.data_file.read_exact(&mut value).ok()?;

        Some(value)
    }

    fn write_entry(buf_writer: &mut BufWriter<File>, value: Option<Vec<u8>>) -> u64 {

        if let Some(value) = value {
            let value_len = value.len() as u32;

            // [value_length][value]
            buf_writer.write_all(&value_len.to_be_bytes()).unwrap();
            buf_writer.write_all(&value).unwrap();

            4 + value.len() as u64
        } else {
            let tombstone_marker = 0xFFFFFFFF_u32;
            buf_writer.write_all(&tombstone_marker.to_be_bytes()).unwrap();
            4
        }
    }

    fn write_index(buf_writer: &mut BufWriter<File>, index: &Vec<(Vec<u8>, u64)>)
        -> (Vec<u8>, Vec<u8>) {
        let min_key = index.first().unwrap().0.clone();
        let max_key = index.last().unwrap().0.clone();
        for (key, offset) in index.into_iter() {
            let key_len = key.len() as u32;
            buf_writer.write_all(&key_len.to_be_bytes()).unwrap();  // 4 bytes
            buf_writer.write_all(&key).unwrap();
            buf_writer.write_all(&offset.to_be_bytes()).unwrap();  // 8 bytes
        }
        (min_key, max_key)
    }
}