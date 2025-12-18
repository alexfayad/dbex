mod memtable;
mod ss_table;

use std::fs;
use std::mem::take;
// src/lib.rs
use crate::memtable::MemTable;
use crate::ss_table::SSTable;

pub struct DBex {
    memtable: MemTable,
    immutable_memtable: Option<MemTable>,
    pre_compact_ss_tables: Vec<SSTable>, // these have overlap in the keys
    compacted_ss_tables: Vec<SSTable>, // no overlap in the keys
    record_count: usize,
}

impl DBex {
    pub fn new() -> Self {
        DBex {
            memtable: MemTable::new(),
            immutable_memtable: None,
            pre_compact_ss_tables: Vec::new(),
            compacted_ss_tables: Vec::new(),
            record_count: 0,
        }
    }

    pub fn memtable(&self) -> &MemTable {
        &self.memtable
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) {

        let key = key.clone();
        let value = value.clone();

        self.memtable.insert(key, value);

        if self.memtable.size_byte() >= 64 * 1024 * 1024  {
            self.flush();
        }

        self.record_count += 1;
    }

    pub fn find(&mut self, key: &[u8]) -> Option<Vec<u8>> {
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
        for ss_table in &mut self.pre_compact_ss_tables {
            let min_key = ss_table.min_key();
            let max_key = ss_table.max_key();

            if key >= min_key && key <= max_key {
                if let Some(value) = ss_table.get(key) {
                    return Some(value);
                }
            }
        }

        // 4. Check Compacted SSTables (Traverse the tree structure)
        for ss_table in &mut self.compacted_ss_tables {
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
            let ss_table = SSTable::new(&table);
            self.pre_compact_ss_tables.push(ss_table);
        }

        // Clear it after flush
        self.immutable_memtable = None;

        // Check if pre_compact_ss_tables is too big now
        if self.pre_compact_ss_tables.len() > 10 {
            self.compact()
        }
    }

    // Delete all SSTables associated with the DB
    pub fn purge(&mut self) {
        for ss_table in &mut self.pre_compact_ss_tables {
            fs::remove_file(ss_table.data_path()).ok();
            fs::remove_file(ss_table.index_path()).ok();
        }
        self.pre_compact_ss_tables.clear();
        self.record_count = 0;
    }

    pub fn start_txn(&mut self) {
        unimplemented!("This function is not yet implemented.")
    }

    pub fn commit_txn(&mut self) {
        // Write to WAL or some shit
        // then flush or some shit
        unimplemented!("This function is not yet implemented.")
    }

    pub fn num_of_ss_tables(&self) -> usize {
        self.pre_compact_ss_tables.len() + self.compacted_ss_tables.len()
    }

    fn compact(&mut self) {
        // Take ownership of pre_compact tables (leaves empty Vec behind)
        let tables_to_compact = take(&mut self.pre_compact_ss_tables);

        self.compacted_ss_tables.extend(tables_to_compact);

        // Question now is how do I compact the tables...
        // We need to recursively compact each ss_table if it's children ss_tables become > 10

    }
}