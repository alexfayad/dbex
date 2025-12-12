// Integration tests for DBex functionality
use dbex::DBex;
use std::fs;

// Helper function to cleanup both db and index files
fn cleanup(path: &str) {
    fs::remove_file(path).ok();
    fs::remove_file(format!("{}.index", path)).ok();
}

// Test guard that ensures cleanup happens even if test panics
struct TestDb {
    path: String,
    db: DBex,
}

impl TestDb {
    fn new(name: &str) -> Self {
        let path = format!("test_{}.db", name);
        // Clean up any leftover files from previous failed runs
        cleanup(&path);

        let db = DBex::new(&path);
        TestDb { path, db }
    }

    // Allow mutable access to the inner database
    fn db(&mut self) -> &mut DBex {
        &mut self.db
    }

    // Get the path (useful for recovery tests)
    fn path(&self) -> &str {
        &self.path
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // This runs even if the test panics!
        cleanup(&self.path);
    }
}

#[test]
fn test_basic_insert_and_find() {
    let mut test_db = TestDb::new("basic");
    let db = test_db.db();

    db.insert(b"key1", b"value1");
    db.insert(b"key2", b"value2");

    assert_eq!(db.find(b"key1"), Some(b"value1".to_vec()));
    assert_eq!(db.find(b"key2"), Some(b"value2".to_vec()));

    // Cleanup happens automatically via Drop
}

#[test]
fn test_find_nonexistent_key() {
    let mut test_db = TestDb::new("nonexistent");
    let db = test_db.db();

    db.insert(b"existing", b"value");

    assert_eq!(db.find(b"nonexistent"), None);
}

#[test]
fn test_empty_database() {
    let mut test_db = TestDb::new("empty");
    let db = test_db.db();

    assert_eq!(db.len(), 0);
    assert_eq!(db.find(b"any_key"), None);
}

#[test]
fn test_overwrite_key() {
    let mut test_db = TestDb::new("overwrite");
    let db = test_db.db();

    db.insert(b"key", b"original_value");
    db.insert(b"key", b"new_value");

    // Since append-only, latest value should be returned
    assert_eq!(db.find(b"key"), Some(b"new_value".to_vec()));
}

#[test]
fn test_large_values() {
    let mut test_db = TestDb::new("large_values");
    let db = test_db.db();

    let large_value = vec![42u8; 1024 * 1024]; // 1MB value
    db.insert(b"large", &large_value);

    assert_eq!(db.find(b"large"), Some(large_value));
}

#[test]
fn test_empty_key_and_value() {
    let mut test_db = TestDb::new("empty_kv");
    let db = test_db.db();

    db.insert(b"", b"empty_key");
    db.insert(b"empty_value", b"");

    assert_eq!(db.find(b""), Some(b"empty_key".to_vec()));
    assert_eq!(db.find(b"empty_value"), Some(b"".to_vec()));
}

#[test]
fn test_binary_data() {
    let mut test_db = TestDb::new("binary");
    let db = test_db.db();

    let binary_key = b"\x00\x01\x02\xFF";
    let binary_value = b"\xDE\xAD\xBE\xEF";

    db.insert(binary_key, binary_value);
    assert_eq!(db.find(binary_key), Some(binary_value.to_vec()));
}

#[test]
fn test_multiple_inserts_and_len() {
    let mut test_db = TestDb::new("len");
    let db = test_db.db();

    assert_eq!(db.len(), 0);

    db.insert(b"key1", b"value1");
    assert_eq!(db.len(), 1);

    db.insert(b"key2", b"value2");
    assert_eq!(db.len(), 2);

    db.insert(b"key3", b"value3");
    assert_eq!(db.len(), 3);

    // Overwriting shouldn't increase len
    db.insert(b"key1", b"new_value");
    assert_eq!(db.len(), 3);
}

#[test]
fn test_flush() {
    let mut test_db = TestDb::new("flush");
    let db = test_db.db();

    db.insert(b"key", b"value");
    db.flush();

    // After flush, data should be on disk
    // We can verify by checking file exists and has content
    let metadata = fs::metadata(test_db.path()).unwrap();
    assert!(metadata.len() > 0);
}

#[test]
fn test_many_small_keys() {
    let mut test_db = TestDb::new("many_small");
    let db = test_db.db();

    let count = 10_000;
    for i in 0..count {
        let key = format!("key_{}", i);
        let value = format!("value_{}", i);
        db.insert(key.as_bytes(), value.as_bytes());
    }

    assert_eq!(db.len(), count);

    // Verify some random entries
    assert_eq!(db.find(b"key_0"), Some(b"value_0".to_vec()));
    assert_eq!(db.find(b"key_5000"), Some(b"value_5000".to_vec()));
    assert_eq!(db.find(b"key_9999"), Some(b"value_9999".to_vec()));
}

#[test]
fn test_index_persistence_and_recovery() {
    let path = "test_index_recovery.db";

    // Manual cleanup at start for recovery test
    cleanup(path);

    // Phase 1: Create database, insert data, and save index
    {
        let mut db = DBex::new(path);
        db.insert(b"key1", b"value1");
        db.insert(b"key2", b"value2");
        db.insert(b"key3", b"value3");
        db.flush(); // This should save the index

        assert_eq!(db.len(), 3);
        assert_eq!(db.find(b"key2"), Some(b"value2".to_vec()));
    } // db goes out of scope, files remain

    // Verify index file was created
    assert!(fs::metadata(format!("{}.index", path)).is_ok());

    // Phase 2: Reopen database and verify data is accessible via loaded index
    {
        let mut db = DBex::new(path);

        // Index should be loaded from file
        assert_eq!(db.len(), 3);

        // All data should be accessible
        assert_eq!(db.find(b"key1"), Some(b"value1".to_vec()));
        assert_eq!(db.find(b"key2"), Some(b"value2".to_vec()));
        assert_eq!(db.find(b"key3"), Some(b"value3".to_vec()));
    }

    // Cleanup at end
    cleanup(path);
}

#[test]
fn test_recovery_with_new_data_after_reopen() {
    let path = "test_recovery_new_data.db";

    // Manual cleanup at start for recovery test
    cleanup(path);

    // Create initial data
    {
        let mut db = DBex::new(path);
        db.insert(b"old_key1", b"old_value1");
        db.insert(b"old_key2", b"old_value2");
        db.flush();
    }

    // Reopen and add more data
    {
        let mut db = DBex::new(path);

        // Old data should be accessible
        assert_eq!(db.find(b"old_key1"), Some(b"old_value1".to_vec()));

        // Add new data
        db.insert(b"new_key1", b"new_value1");
        db.insert(b"new_key2", b"new_value2");
        db.flush();

        assert_eq!(db.len(), 4);
    }

    // Reopen again and verify everything
    {
        let mut db = DBex::new(path);
        assert_eq!(db.len(), 4);
        assert_eq!(db.find(b"old_key1"), Some(b"old_value1".to_vec()));
        assert_eq!(db.find(b"old_key2"), Some(b"old_value2".to_vec()));
        assert_eq!(db.find(b"new_key1"), Some(b"new_value1".to_vec()));
        assert_eq!(db.find(b"new_key2"), Some(b"new_value2".to_vec()));
    }

    // Cleanup at end
    cleanup(path);
}
