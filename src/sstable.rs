use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::memtable::MemTable;

pub struct SSTable {
    data_path: PathBuf,
    index_path: PathBuf,
    index: HashMap<Vec<u8>, u64>,
    min_key: Vec<u8>,
    max_key: Vec<u8>,
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

        let mut offset: u64 = 0;
        let mut index_vec = Vec::new();

        for (key, value) in memtable.data() {
            // Save index entry (key → current offset)
            index_vec.push((key.clone(), offset));

            offset += Self::write_entry(&mut data_bufwriter, value);
        }

        let (min_key, max_key) = SSTable::write_index(&mut index_bufwriter, &index_vec);

        // Build in-memory HashMap from the same data
        let mut index = HashMap::new();
        for (key, offset) in &index_vec {
            index.insert(key.clone(), offset.clone());
        }

        let data_file = data_bufwriter.into_inner().unwrap();
        let index_file = index_bufwriter.into_inner().unwrap();

        data_file.sync_all().unwrap();
        index_file.sync_all().unwrap();

        SSTable {
            data_path,
            index_path,
            index,
            min_key,
            max_key,
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<Vec<u8>> {
        if let Some(offset) = self.index.get(key) {
            return self.read_value_at_offset(*offset);
        }
        None  // Key not found in index
    }

    pub fn get_from_index_file(&self, key: &[u8]) -> Option<Vec<u8>> {
        // Fetch index file
        let index_file = File::open(&self.index_path).unwrap();
        let mut reader = BufReader::new(index_file);

        loop {
            // Read key length (4 bytes)
            let mut key_len_bytes = [0u8; 4];
            if reader.read_exact(&mut key_len_bytes).is_err() {
                break;
            }
            let key_len = u32::from_be_bytes(key_len_bytes) as usize;

            // Read key
            let mut stored_key = vec![0u8; key_len];
            reader.read_exact(&mut stored_key).ok()?;

            // Read offset (8 bytes)
            let mut offset_bytes = [0u8; 8];
            reader.read_exact(&mut offset_bytes).ok()?;
            let offset = u64::from_be_bytes(offset_bytes);

            // Check if this is the key we're looking for
            if &stored_key == key {
                // Found it! Now use the offset to read from data file
                return self.read_value_at_offset(offset);
            }
        }
        None
    }

    pub fn data_path (&self) -> &PathBuf {
        &self.data_path
    }

    pub fn index_path (&self) -> &PathBuf {
        &self.index_path
    }

    fn read_value_at_offset(&self, offset: u64) -> Option<Vec<u8>> {
        let mut data_file = File::open(&self.data_path).ok()?;
        data_file.seek(SeekFrom::Start(offset)).ok()?;

        // Read value length
        let mut len_bytes = [0u8; 4];
        data_file.read_exact(&mut len_bytes).ok()?;
        let value_len = u32::from_be_bytes(len_bytes) as usize;

        if value_len == 0xFFFFFFFF {
            return None;  // This key was deleted
        }

        // Read value
        let mut value = vec![0u8; value_len];
        data_file.read_exact(&mut value).ok()?;

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