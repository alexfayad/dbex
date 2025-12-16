// Integration tests for DBex functionality
use dbex::DBex;

// Test guard that ensures cleanup happens even if test panics
pub struct TestDb {
    db: DBex,
}

impl TestDb {
    pub fn new() -> Self {
        let db = DBex::new();
        TestDb {
            db
        }
    }

    // Allow mutable access to the inner database
    pub fn db(&mut self) -> &mut DBex {
        &mut self.db
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // This runs even if the test panics!
        self.db.purge();
    }
}