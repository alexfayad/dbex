pub mod memtable;
pub mod ss_table;
pub mod write_ahead_log;
pub mod utils;


use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fs;
use std::io::{Seek, SeekFrom};
use std::mem::take;

// src/lib.rs
use crate::memtable::MemTable;
use crate::ss_table::SSTable;
use crate::utils::Operation::{Delete, Insert};
use crate::write_ahead_log::WriteAheadLog;

pub struct DBex {
    memtable: MemTable,
    immutable_memtable: Option<MemTable>,
    l0_ss_tables: Vec<SSTable>,
    l1_ss_tables: Vec<SSTable>,
    l2_ss_tables: Vec<SSTable>,
    write_ahead_log: WriteAheadLog,
    is_in_txn: bool,
    record_count: u64,
    lsn: u64,
}

impl DBex {
    pub fn new() -> Self {
        fs::create_dir_all("db_data/wals").unwrap();
        fs::create_dir_all("db_data/ss_tables").unwrap();
        DBex {
            memtable: MemTable::new(),
            immutable_memtable: None,
            l0_ss_tables: Vec::new(),
            l1_ss_tables: Vec::new(),
            l2_ss_tables: Vec::new(),
            write_ahead_log: WriteAheadLog::new(),
            is_in_txn: false,
            record_count: 0,
            lsn: 0,
        }
    }

    pub fn memtable(&self) -> &MemTable {
        &self.memtable
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {

        // self.write_ahead_log.write(Insert, self.lsn.clone(), Some(key.clone()), Some(value.clone()));

        self.memtable.insert(key, value);

        if self.memtable.size_byte() >= 64 * 1024 * 1024  {
            self.flush();
        }

        self.record_count += 1;
        self.lsn += 1;
    }

    pub fn remove(&mut self, key: &Vec<u8>) {
        let key = key.to_vec();

        // self.write_ahead_log.write(Delete, self.lsn.clone(), Some(key.clone()), None);

        self.memtable.remove(&key);

        self.record_count -= 1;
        self.lsn += 1;
    }

    pub fn find(&mut self, key: &Vec<u8>) -> Option<Vec<u8>> {
        // 1. Check active MemTable (RAM)
        if let Some(value) = self.memtable.get(key) {
            return Some(value.clone());
        }

        // 2. Check immutable MemTable (if being flushed)
        if let Some(ref table) = self.immutable_memtable {
            if let Some(value) = table.get(key) {
                return Some(value.clone());
            }
        }

        // 3. Check Pre Compacted SSTables (newest to oldest)
        for ss_table in &mut self.l0_ss_tables {
            let min_key = ss_table.min_key();
            let max_key = ss_table.max_key();

            if key >= min_key && key <= max_key {
                if let Some(value) = ss_table.get(key) {
                    return Some(value);
                }
            }
        }

        // 4. Check Compacted SSTables (Traverse the tree structure)
        for ss_table in &mut self.l1_ss_tables {
            let min_key = ss_table.min_key();
            let max_key = ss_table.max_key();

            if key >= min_key && key <= max_key {
                if let Some(value) = ss_table.get(key) {
                    return Some(value);
                }
            }
        }

        for ss_table in &mut self.l2_ss_tables {
            let min_key = ss_table.min_key();
            let max_key = ss_table.max_key();

            if key >= min_key && key <= max_key {
                if let Some(value) = ss_table.get(key) {
                    return Some(value);
                }
            }
        }

        None  // Not found
    }

    pub fn flush(&mut self) {
        // Move current memtable to immutable
        self.immutable_memtable = Some(std::mem::replace(&mut self.memtable, MemTable::new()));

        // Flush the immutable one
        if let Some(ref table) = self.immutable_memtable {
            let mut ss_table = SSTable::new();
            ss_table.load_from_memtable(&table);
            self.l0_ss_tables.push(ss_table);
        }

        // Clear it after flush
        self.immutable_memtable = None;

        // Check if pre_compact_ss_tables is too big now
        if self.l0_ss_tables.len() > 10 {
            self.compact_l0()
        }
        // Check if pre_compact_ss_tables is too big now
        if self.l1_ss_tables.len() > 10 {
            self.compact_l1()
        }
    }

    // Delete all SSTables associated with the DB
    pub fn purge(&mut self) {
        for ss_table in &mut self.l0_ss_tables {
            fs::remove_dir_all("db_data/").ok();
        }
        self.l0_ss_tables.clear();
        self.l1_ss_tables.clear();
        self.l2_ss_tables.clear();
        self.record_count = 0;
    }

    pub fn start_txn(&mut self) {
        self.is_in_txn = true;
    }

    pub fn commit_txn(&mut self) {
        // Write to WAL or some shit
        // then flush or some shit
        self.flush();
        self.is_in_txn = false;
    }

    pub fn cnt_of_l0_ss_tables(&self) -> usize {
        self.l0_ss_tables.len()
    }

    pub fn cnt_of_l1_ss_tables(&self) -> usize {
        self.l1_ss_tables.len()
    }

    pub fn cnt_of_l2_ss_tables(&self) -> usize {
        self.l2_ss_tables.len()
    }

    fn compact_l0(&mut self) {
        // take() Takes ownership of pre_compact tables (leaves empty Vec behind)
        let mut tables_to_compact: Vec<SSTable> = take(&mut self.l0_ss_tables);
        let mut new_ss_table = SSTable::new();
        let mut new_ss_table_offset = 0;
        let mut new_indexes = Vec::new();

        let mut min_vals = BinaryHeap::new();

        for (ss_table_idx, ss_table) in tables_to_compact.iter_mut().enumerate() {
            ss_table.index_reader_mut().seek(SeekFrom::Start(0)).unwrap();
            let (stored_key, data_file_offset) = match ss_table.get_next_key_in_index_file() {
                Some(data) => data,
                None => {
                    panic!("Error Empty SSTable found. SSTable index: {}, data path: {:?}, index path: {:?}",
                        ss_table_idx,
                        ss_table.data_path(),
                        ss_table.index_path()
                    );
                }
            };
            min_vals.push(Reverse((stored_key, ss_table_idx, data_file_offset)));
        }

        let mut last_seen_key: Option<Vec<u8>> = None;

        while !min_vals.is_empty() {
            let Reverse((
                    stored_key,
                    ss_table_idx,
                    data_file_offset
                )) = min_vals.pop().unwrap();

            if last_seen_key.as_ref() == Some(&stored_key) {
                continue;
            }
            last_seen_key = Some(stored_key.clone());

            let ss_table = tables_to_compact.get_mut(ss_table_idx).unwrap();

            let value = ss_table.read_value_at_offset(data_file_offset);
            if value.is_none() {
                continue;
            }

            new_ss_table.write_entry(&value);
            new_indexes.push((stored_key.clone(), new_ss_table_offset));

            let value_len = value.unwrap().len();
            new_ss_table_offset += 4 + value_len as u64;

            let next_stored_key = ss_table.get_next_key_in_index_file();
            if next_stored_key.is_none() {
                continue;
            }
            let (next_stored_key, next_data_file_offset) = next_stored_key.unwrap();

            min_vals.push(Reverse((next_stored_key, ss_table_idx, next_data_file_offset)));
        }

        new_ss_table.write_index(&new_indexes);
        self.l1_ss_tables.push(new_ss_table);
    }

    fn compact_l1(&mut self) {
        // take() Takes ownership of pre_compact tables (leaves empty Vec behind)
        let mut tables_to_compact: Vec<SSTable> = take(&mut self.l1_ss_tables);
        let mut new_ss_table = SSTable::new();
        let mut new_ss_table_offset = 0;
        let mut new_indexes = Vec::new();

        let mut min_vals = BinaryHeap::new();

        for (ss_table_idx, ss_table) in tables_to_compact.iter_mut().enumerate() {
            ss_table.index_reader_mut().seek(SeekFrom::Start(0)).unwrap();
            let (stored_key, data_file_offset) = match ss_table.get_next_key_in_index_file() {
                Some(data) => data,
                None => {
                    panic!("Error Empty SSTable found. SSTable index: {}, data path: {:?}, index path: {:?}",
                           ss_table_idx,
                           ss_table.data_path(),
                           ss_table.index_path()
                    );
                }
            };
            min_vals.push(Reverse((stored_key, ss_table_idx, data_file_offset)));
        }

        let mut last_seen_key: Option<Vec<u8>> = None;

        while !min_vals.is_empty() {
            let Reverse((
                            stored_key,
                            ss_table_idx,
                            data_file_offset
                        )) = min_vals.pop().unwrap();

            if last_seen_key.as_ref() == Some(&stored_key) {
                continue;
            }
            last_seen_key = Some(stored_key.clone());

            let ss_table = tables_to_compact.get_mut(ss_table_idx).unwrap();

            let value = ss_table.read_value_at_offset(data_file_offset);
            if value.is_none() {
                continue;
            }

            new_ss_table.write_entry(&value);
            new_indexes.push((stored_key.clone(), new_ss_table_offset));

            let value_len = value.unwrap().len();
            new_ss_table_offset += 4 + value_len as u64;

            let next_stored_key = ss_table.get_next_key_in_index_file();
            if next_stored_key.is_none() {
                continue;
            }
            let (next_stored_key, next_data_file_offset) = next_stored_key.unwrap();

            min_vals.push(Reverse((next_stored_key, ss_table_idx, next_data_file_offset)));
        }

        new_ss_table.write_index(&new_indexes);
        self.l2_ss_tables.push(new_ss_table);
    }
}