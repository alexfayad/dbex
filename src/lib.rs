use std::collections::HashMap;

mod bson;
pub use bson::{encode_document, decode_document};

/// BSON value types
#[derive(Debug, Clone, PartialEq)]
pub enum BsonValue {
    Double(f64),
    String(String),
    Document(Box<Document>),  // Box to avoid infinite size
    Array(Vec<BsonValue>),
    Binary(Vec<u8>),
    Boolean(bool),
    Null,
    Int32(i32),
    Int64(i64),
    // Add more types as needed: DateTime, Timestamp, etc.
}

/// BSON Document - a map of string keys to BSON values
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub id: u64,
    pub data: HashMap<String, BsonValue>
}

impl Document {
    pub fn new() -> Self {
        Document { id: 0, data: HashMap::new() }
    }

    pub fn insert(&mut self, key: String, value: BsonValue) {
        self.data.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&BsonValue> {
        self.data.get(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &BsonValue)> {
        self.data.iter()
    }
}

pub type Query = Document;

pub struct DBex {
    data: HashMap<u64, Document>,
    storage_path: String,
    next_id: u64,
}

impl DBex {
    pub fn new(_storage_path: &str) -> Self {
        // Ensure .dbex extension
        let storage_path = if _storage_path.ends_with(".dbex") {
            _storage_path.to_string()
        } else {
            format!("{}.dbex", _storage_path)
        };
        
        let mut db = DBex {
            data: HashMap::new(),
            storage_path,
            next_id: 1,
        };
        db.load(); // Load existing data if file exists
        db
    }

    pub fn insert(&mut self, mut _document: Document) -> u64 {
        let id = self.next_id;
        _document.id = id;
        self.data.insert(id, _document);
        self.save();
        self.next_id += 1;
        id
    }

    pub fn find_by_id(&self, _id: &u64) -> Option<Document> {
        self.data.get(_id).cloned()
    }

    pub fn find_all(&self) -> Vec<Document> {
        self.data
            .values()
            .cloned()
            .collect()
    }

    pub fn find(&self, _query: &Query) -> Vec<Document> {
        self.data.values()
            .filter(|doc: &&Document| doc.data == _query.data)
            .cloned()
            .collect()
    }

    pub fn update_by_id(&mut self, _id: &u64, _updates: &Document) -> usize {
        let count = self.data.get_mut(_id).map(|doc: &mut Document| {
            for (key, value) in _updates.iter() {
                doc.data.insert(key.clone(), value.clone());
            }
            1
        }).unwrap_or_else(|| 0);
        self.save();
        count
    }

    pub fn update(&mut self, _query: &Query, _updates: &Document) -> usize {
        let mut count = 0;
        for (_, doc) in self.data.iter_mut() {
            if doc.data == _query.data {
                for (key, value) in _updates.iter() {
                    doc.insert(key.clone(), value.clone());
                }
                count += 1;
            }
        }
        self.save();
        count
    }

    pub fn delete_by_id(&mut self, _id: &u64) -> usize {
        let count = self.data.remove(_id).is_some() as usize;
        self.save();
        count
    }

    pub fn delete(&mut self, _query: &Query) -> usize {
        let ids_to_delete: Vec<u64> = self
            .find(_query)
            .iter()
            .map(|doc: &Document| doc.id)
            .collect();

        for id in ids_to_delete.iter() {
            self.delete_by_id(id);
        }
        self.save();
        ids_to_delete.len() as usize
    }

    fn save(&self) {
        if let Some(parent) = std::path::Path::new(&self.storage_path).parent() {
            std::fs::create_dir_all(parent).unwrap_or_default();
        }
        
        let mut file_data = Vec::new();
        
        for (_, doc) in &self.data {
            let encoded = encode_document(doc);
            file_data.extend_from_slice(&encoded);
        }
        
        std::fs::write(&self.storage_path, file_data).unwrap();
    }

    fn load(&mut self) {
        if std::path::Path::new(&self.storage_path).exists() {
            let data = std::fs::read(&self.storage_path).unwrap_or_default();

            if data.len() == 0 {
                println!("No data found in storage file.");
                return;
            }

            if let Ok(doc) = decode_document(&data) {
                self.insert(doc);
            } else {
                panic!("Unable to decode document from storage.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Helper to get test database path
    fn test_db_path(name: &str) -> String {
        format!("test_data/{}", name)
    }

    // Helper to delete test database
    fn test_delete_db(name: &str) {
        if std::path::Path::new(&test_db_path(name)).exists() {
            let _ = std::fs::remove_file(&test_db_path(name));
        }
    }

    #[test]
    fn test_encode_decode() {
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("test".to_string()));
        let encoded = encode_document(&doc);
        let decoded = decode_document(&encoded).unwrap_or_else(|e| panic!("Unable to decode document: {}", e));
        assert_eq!(doc, decoded);
    }

    #[test]
    fn test_new() {
        let _db = DBex::new(&test_db_path("test_db"));
    }

    #[test]
    fn test_insert() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut document = Document::new();
        document.insert("name".to_string(), BsonValue::String("test".to_string()));
        println!("document: {:?}", document);
        let id = db.insert(document);
        assert_ne!(id, 0);
        assert_eq!(db.delete_by_id(&id), 1);
        assert!(db.find_by_id(&id).is_none());
    }

    #[test]
    fn test_find_by_id() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut document = Document::new();
        document.insert("name".to_string(), BsonValue::String("test".to_string()));
        let id = db.insert(document.clone());
        
        let found = db.find_by_id(&id);
        assert!(found.is_some());
        // Uncomment once implementation is complete
        // assert_eq!(found.unwrap(), document);
    }

    #[test]
    fn test_find_by_id_not_found() {
        test_delete_db("test_db");
        let db = DBex::new(&test_db_path("test_db"));
        let found = db.find_by_id(&0);
        assert!(found.is_none());
    }

    #[test]
    fn test_find_all() {
        test_delete_db("test_db_find_all");
        let mut db = DBex::new(&test_db_path("test_db_find_all"));
        let mut doc1 = Document::new();
        doc1.insert("name".to_string(), BsonValue::String("doc1".to_string()));
        let mut doc2 = Document::new();
        doc2.insert("name".to_string(), BsonValue::String("doc2".to_string()));
        
        db.insert(doc1);
        db.insert(doc2);
        
        let all = db.find_all();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_find_all_empty() {
        test_delete_db("test_db");
        let db = DBex::new(&test_db_path("test_db"));
        let all = db.find_all();
        assert_eq!(all.len(), 0);
    }

    #[test]
    fn test_find() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc1 = Document::new();
        doc1.insert("name".to_string(), BsonValue::String("doc1".to_string()));
        let mut doc2 = Document::new();
        doc2.insert("name".to_string(), BsonValue::String("doc2".to_string()));
        
        db.insert(doc1);
        db.insert(doc2);
        
        let mut query = Query::new();
        query.insert("name".to_string(), BsonValue::String("doc1".to_string()));
        let results = db.find(&query);
        // Add specific assertions once query format is defined
        let _ = results.len();
    }

    #[test]
    fn test_find_no_matches() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("test".to_string()));
        db.insert(doc);
        
        let mut query = Query::new();
        query.insert("name".to_string(), BsonValue::String("nonexistent".to_string()));
        let results = db.find(&query);
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_update() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("original".to_string()));
        let _id = db.insert(doc);
        
        let mut query = Query::new();
        query.insert("name".to_string(), BsonValue::String("original".to_string()));
        let mut updates = Document::new();
        updates.insert("name".to_string(), BsonValue::String("updated".to_string()));
        let _count = db.update(&query, &updates);
        // Add assertions once implementation is complete
    }

    #[test]
    fn test_update_no_matches() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("test".to_string()));
        db.insert(doc);
        
        let mut query = Query::new();
        query.insert("name".to_string(), BsonValue::String("nonexistent".to_string()));
        let mut updates = Document::new();
        updates.insert("name".to_string(), BsonValue::String("updated".to_string()));
        let count = db.update(&query, &updates);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_update_by_id() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("original".to_string()));
        let id = db.insert(doc);
        
        let mut updates = Document::new();
        updates.insert("name".to_string(), BsonValue::String("updated".to_string()));
        updates.insert("age".to_string(), BsonValue::Int32(30));
        
        let count = db.update_by_id(&id, &updates);
        assert_eq!(count, 1, "Should update 1 document");
        
        let updated_doc = db.find_by_id(&id).expect("Document should exist");
        assert_eq!(
            updated_doc.get("name"),
            Some(&BsonValue::String("updated".to_string())),
            "Name should be updated"
        );
        assert_eq!(
            updated_doc.get("age"),
            Some(&BsonValue::Int32(30)),
            "Age should be added"
        );
    }

    #[test]
    fn test_update_by_id_not_found() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut updates = Document::new();
        updates.insert("name".to_string(), BsonValue::String("updated".to_string()));
        
        let nonexistent_id = 999u64;
        let count = db.update_by_id(&nonexistent_id, &updates);
        assert_eq!(count, 0, "Should return 0 when document not found");
    }

    #[test]
    fn test_delete_by_id() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("to_delete".to_string()));
        let id = db.insert(doc);
        
        let count = db.delete_by_id(&id);
        assert_eq!(count, 1, "Should delete 1 document");
        
        let deleted = db.find_by_id(&id);
        assert!(deleted.is_none(), "Document should be deleted");
    }

    #[test]
    fn test_delete_by_id_not_found() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let nonexistent_id = 999u64;
        
        let count = db.delete_by_id(&nonexistent_id);
        assert_eq!(count, 0, "Should return 0 when document not found");
    }

    #[test]
    fn test_delete() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("to_delete".to_string()));
        db.insert(doc);
        
        let mut query = Query::new();
        query.insert("name".to_string(), BsonValue::String("to_delete".to_string()));
        let _count = db.delete(&query);
        // Add assertions once implementation is complete
    }

    #[test]
    fn test_delete_no_matches() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc = Document::new();
        doc.insert("name".to_string(), BsonValue::String("test".to_string()));
        db.insert(doc);
        
        let mut query = Query::new();
        query.insert("name".to_string(), BsonValue::String("nonexistent".to_string()));
        let count = db.delete(&query);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_delete_all() {
        test_delete_db("test_db");
        let mut db = DBex::new(&test_db_path("test_db"));
        let mut doc1 = Document::new();
        doc1.insert("name".to_string(), BsonValue::String("doc1".to_string()));
        let mut doc2 = Document::new();
        doc2.insert("name".to_string(), BsonValue::String("doc1".to_string()));
        
        db.insert(doc1);
        db.insert(doc2);
        
        // Assuming a query that matches all documents
        let mut query = Query::new();
        query.insert("name".to_string(), BsonValue::String("doc1".to_string()));
        let count = db.delete(&query);
        assert_eq!(count, 2, "Should delete 2 documents");
        assert_eq!(db.find_all().len(), 0, "Should have 0 documents");
    }

    #[test]
    fn test_persistence() {
        test_delete_db("test_persistence_db");
        let storage_path = test_db_path("test_persistence_db"); // Will become "test_data/test_persistence_db.dbex"
        {
            let mut db = DBex::new(&storage_path);
            let mut doc = Document::new();
            doc.insert("name".to_string(), BsonValue::String("persistent".to_string()));
            db.insert(doc);
        }
        
        let db = DBex::new(&storage_path);
        let all = db.find_all();
        
        assert_eq!(all.len(), 1, "Should have 1 persisted document");
        assert_eq!(all[0].id, 1, "Document ID should be 1");
        assert_eq!(
            all[0].get("name"),
            Some(&BsonValue::String("persistent".to_string())),
            "Document should have name='persistent'"
        );
    }
}
