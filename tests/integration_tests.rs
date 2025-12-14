// Integration tests for DBex functionality
use dbex::DBex;
use std::fs;
use std::path::Path;

// Test guard that ensures cleanup happens even if test panics
struct TestDb {
    name: String,
    db: DBex,
}

impl TestDb {
    fn new(name: &str) -> Self {
        let db = DBex::new();
        TestDb {
            name: name.to_string(),
            db
        }
    }

    // Allow mutable access to the inner database
    fn db(&mut self) -> &mut DBex {
        &mut self.db
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // This runs even if the test panics!
        self.db.purge();
    }
}

#[test]
fn test_basic_insert_and_find() {
    let mut test_db = TestDb::new("basic");
    let db = test_db.db();

    db.insert(b"key1".to_vec(), b"value1".to_vec());
    db.insert(b"key2".to_vec(), b"value2".to_vec());

    assert_eq!(db.find(b"key1"), Some(b"value1".to_vec()));
    assert_eq!(db.find(b"key2"), Some(b"value2".to_vec()));
}

#[test]
fn test_find_nonexistent_key() {
    let mut test_db = TestDb::new("nonexistent");
    let db = test_db.db();

    db.insert(b"existing".to_vec(), b"value".to_vec());

    assert_eq!(db.find(b"nonexistent"), None);
}

#[test]
fn test_empty_database() {
    let mut test_db = TestDb::new("empty");
    let db = test_db.db();

    assert_eq!(db.memtable().len(), 0);
    assert_eq!(db.find(b"any_key"), None);
}

#[test]
fn test_overwrite_key() {
    let mut test_db = TestDb::new("overwrite");
    let db = test_db.db();

    db.insert(b"key".to_vec(), b"original_value".to_vec());
    db.insert(b"key".to_vec(), b"new_value".to_vec());

    // Since append-only, latest value should be returned
    assert_eq!(db.find(b"key"), Some(b"new_value".to_vec()));
}

#[test]
fn test_large_values() {
    let mut test_db = TestDb::new("large_values");
    let db = test_db.db();

    let large_value = vec![42u8; 1024 * 1024]; // 1MB value
    db.insert(b"large".to_vec(), large_value.clone());

    assert_eq!(db.find(b"large"), Some(large_value));
}

#[test]
fn test_empty_key_and_value() {
    let mut test_db = TestDb::new("empty_kv");
    let db = test_db.db();

    db.insert(b"".to_vec(), b"empty_key".to_vec());
    db.insert(b"empty_value".to_vec(), b"".to_vec());

    assert_eq!(db.find(b""), Some(b"empty_key".to_vec()));
    assert_eq!(db.find(b"empty_value"), Some(b"".to_vec()));
}

#[test]
fn test_binary_data() {
    let mut test_db = TestDb::new("binary");
    let db = test_db.db();

    let binary_key = b"\x00\x01\x02\xFF";
    let binary_value = b"\xDE\xAD\xBE\xEF";

    db.insert(binary_key.to_vec(), binary_value.to_vec());
    assert_eq!(db.find(binary_key), Some(binary_value.to_vec()));
}

#[test]
fn test_multiple_inserts_and_len() {
    let mut test_db = TestDb::new("len");
    let db = test_db.db();

    assert_eq!(db.memtable().len(), 0);

    db.insert(b"key1".to_vec(), b"value1".to_vec());
    assert_eq!(db.memtable().len(), 1);

    db.insert(b"key2".to_vec(), b"value2".to_vec());
    assert_eq!(db.memtable().len(), 2);

    db.insert(b"key3".to_vec(), b"value3".to_vec());
    assert_eq!(db.memtable().len(), 3);

    // Overwriting shouldn't increase len
    db.insert(b"key1".to_vec(), b"new_value".to_vec());
    assert_eq!(db.memtable().len(), 3);
}

#[test]
fn test_flush() {
    let mut test_db = TestDb::new("flush");
    let db = test_db.db();

    db.insert(b"key".to_vec(), b"value".to_vec());
    db.flush();

    // After flush, data should be in an SSTable
    // Verify we can still read it
    assert_eq!(db.find(b"key"), Some(b"value".to_vec()));
}

#[test]
fn test_many_small_keys() {
    let mut test_db = TestDb::new("many_small");
    let db = test_db.db();

    let count = 10_000;
    for i in 0..count {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        db.insert(key.as_bytes().to_vec(), value.as_bytes().to_vec());
    }

    assert_eq!(db.memtable().len(), count);

    // Verify some random entries
    assert_eq!(db.find(b"key_0"), Some(b"value_0".to_vec()));
    assert_eq!(db.find(b"key_5000"), Some(b"value_5000".to_vec()));
    assert_eq!(db.find(b"key_9999"), Some(b"value_9999".to_vec()));
}

#[test]
fn test_memtable_flush_to_sstable() {
    let mut test_db = TestDb::new("memtable_flush");
    let db = test_db.db();

    // Insert some data
    db.insert(b"key1".to_vec(), b"value1".to_vec());
    db.insert(b"key2".to_vec(), b"value2".to_vec());
    db.insert(b"key3".to_vec(), b"value3".to_vec());

    // Data should be in MemTable
    assert_eq!(db.find(b"key1"), Some(b"value1".to_vec()));

    // Flush to SSTable
    db.flush();

    // Data should still be readable from SSTable
    assert_eq!(db.find(b"key1"), Some(b"value1".to_vec()));
    assert_eq!(db.find(b"key2"), Some(b"value2".to_vec()));
    assert_eq!(db.find(b"key3"), Some(b"value3".to_vec()));
}

#[test]
fn test_read_from_multiple_sstables() {
    let mut test_db = TestDb::new("multiple_sstables");
    let db = test_db.db();

    // Insert and flush first batch
    db.insert(b"batch1_key1".to_vec(), b"batch1_value1".to_vec());
    db.insert(b"batch1_key2".to_vec(), b"batch1_value2".to_vec());
    db.flush();

    // Insert and flush second batch
    db.insert(b"batch2_key1".to_vec(), b"batch2_value1".to_vec());
    db.insert(b"batch2_key2".to_vec(), b"batch2_value2".to_vec());
    db.flush();

    // Should be able to read from both SSTables
    assert_eq!(db.find(b"batch1_key1"), Some(b"batch1_value1".to_vec()));
    assert_eq!(db.find(b"batch2_key1"), Some(b"batch2_value1".to_vec()));
}

#[test]
fn test_newest_value_wins() {
    let mut test_db = TestDb::new("newest_wins");
    let db = test_db.db();

    // Insert and flush old value
    db.insert(b"key".to_vec(), b"old_value".to_vec());
    db.flush();

    // Insert new value (in MemTable)
    db.insert(b"key".to_vec(), b"new_value".to_vec());

    // Should return newest value from MemTable, not SSTable
    assert_eq!(db.find(b"key"), Some(b"new_value".to_vec()));
}
