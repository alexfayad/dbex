use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use rkyv::{Archive, Deserialize, Serialize};
use rkyv::rancor::{Error};
use rkyv::util::AlignedVec;
use crate::utils::Operation;

pub struct WriteAheadLog {
    cur_wal_path: PathBuf,
    cur_wal_file_writer: BufWriter<File>,
    prev_fal_files: Vec<PathBuf>
}

impl WriteAheadLog {
    pub fn new() -> Self {

        std::fs::create_dir_all("wals").unwrap();
        let cur_wal_path = PathBuf::from("wals/cur.wal");

        let wal_file: File;

        if cur_wal_path.exists() {
            wal_file = File::open(&cur_wal_path).unwrap();
        } else {
            wal_file = File::create(&cur_wal_path).unwrap();
        }

        WriteAheadLog{
            cur_wal_path,
            cur_wal_file_writer: BufWriter::new(wal_file),
            prev_fal_files: Vec::new()
        }
    }

    pub fn write(&mut self, operation: Operation, lsn: u64, key: Option<Vec<u8>>, value: Option<Vec<u8>>) {

        let wal_entry = WalEntry::new(
            lsn,
            operation,
            key,
            value
        );

        let encoded_wal_entry: AlignedVec = rkyv::to_bytes::<Error>(&wal_entry).unwrap();
        let data_len = encoded_wal_entry.len();

        // [data_len][encoded_wal_entry]
        self.cur_wal_file_writer.write(&data_len.to_be_bytes()).unwrap();
        self.cur_wal_file_writer.write(encoded_wal_entry.as_slice()).unwrap();
    }

    pub fn read(&mut self, start_offset: u64) -> Vec<WalEntry> {

        let mut wal_entries: Vec<WalEntry> = Vec::new();

        let wal_file = File::open(&self.cur_wal_path).unwrap();

        let mut wal_reader = BufReader::new(wal_file);
        wal_reader.seek(SeekFrom::Start(start_offset)).unwrap();


        loop {
            // Read data length (4 bytes)
            let mut data_len_bytes = [0u8; 8];
            if wal_reader.read_exact(&mut data_len_bytes).is_err() {
                break;
            }
            let data_len = u64::from_be_bytes(data_len_bytes) as usize;

            // Read wal_entry
            let mut encoded_wal_entry_bytes = vec![0u8; data_len];
            if wal_reader.read_exact(&mut encoded_wal_entry_bytes).is_err() {
                break;
            }
            let archived = rkyv::access::<ArchivedWalEntry, Error>(&encoded_wal_entry_bytes).unwrap();
            let wal_entry: WalEntry = rkyv::deserialize::<WalEntry, Error>(archived).unwrap();

            wal_entries.push(wal_entry);
        }

        wal_entries
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, PartialEq)]pub struct WalEntry {
    lsn: u64,
    operation: Operation,
    key: Option<Vec<u8>>,
    value: Option<Vec<u8>>
}

impl WalEntry {
    pub fn new(lsn: u64, operation: Operation, key: Option<Vec<u8>>, value: Option<Vec<u8>>) -> Self {
        WalEntry{
            lsn,
            operation,
            key,
            value
        }
    }
}