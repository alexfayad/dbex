use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Basic JSON database implementation
pub struct SimpleJSONDB {
    data: HashMap<String, serde_json::Value>,
    storage_path: String,
    next_id: u64,
}

impl SimpleJSONDB {
    /// Create a new database instance
    pub fn new(storage_path: &str) -> Self {
        let mut db: SimpleJSONDB = SimpleJSONDB {
            data: HashMap::new(),
            storage_path: storage_path.to_string(),
            next_id: 1u64,
        };
        db.load();
        db
    }

    /// Insert a document
    /// Returns the ID of the inserted document
    pub fn insert(&mut self, mut document: serde_json::Value) -> String {
        // Generate ID if not provided
        let id: String = if let Some(id_val) = document.get("id") {
            // Extract ID as string
            if let Some(id_str) = id_val.as_str() {
                id_str.to_string()
            } else if let Some(id_num) = id_val.as_u64() {
                id_num.to_string()
            } else {
                let new_id: String = self.next_id.to_string();
                self.next_id += 1;
                if let Some(obj) = document.as_object_mut() {
                    obj.insert("id".to_string(), serde_json::Value::String(new_id.clone()));
                }
                new_id
            }
        } else {
            let new_id: String = self.next_id.to_string();
            self.next_id += 1;
            if let Some(obj) = document.as_object_mut() {
                obj.insert("id".to_string(), serde_json::Value::String(new_id.clone()));
            }
            new_id
        };

        self.data.insert(id.clone(), document);
        self.save();
        id
    }

    /// Find a document by ID
    pub fn find_by_id(&self, id: &str) -> Option<serde_json::Value> {
        self.data.get(id).cloned()
    }

    /// Get all documents
    pub fn find_all(&self) -> Vec<serde_json::Value> {
        self.data.values().cloned().collect()
    }

    /// Find documents matching a query
    pub fn find(&self, query: &serde_json::Value) -> Vec<serde_json::Value> {
        let query_obj: &serde_json::Map<String, serde_json::Value> = match query.as_object() {
            Some(obj) => obj,
            None => return vec![],
        };

        self.data
            .values()
            .filter(|doc: &&serde_json::Value| {
                if let Some(doc_obj) = doc.as_object() {
                    query_obj.iter().all(|(key, value): (&String, &serde_json::Value)| {
                        doc_obj.get(key).map_or(false, |v: &serde_json::Value| v == value)
                    })
                } else {
                    false
                }
            })
            .cloned()
            .collect::<Vec<serde_json::Value>>()
    }

    /// Update documents matching a query
    /// Returns number of documents updated
    pub fn update(&mut self, query: &serde_json::Value, updates: &serde_json::Value) -> usize {
        let query_obj: &serde_json::Map<String, serde_json::Value> = match query.as_object() {
            Some(obj) => obj,
            None => return 0,
        };

        let updates_obj: &serde_json::Map<String, serde_json::Value> = match updates.as_object() {
            Some(obj) => obj,
            None => return 0,
        };

        let mut count: usize = 0;
        for doc in self.data.values_mut() {
            if let Some(doc_obj) = doc.as_object_mut() {
                let matches: bool = query_obj.iter().all(|(key, value): (&String, &serde_json::Value)| {
                    doc_obj.get(key).map_or(false, |v: &serde_json::Value| v == value)
                });

                if matches {
                    for (key, value) in updates_obj {
                        doc_obj.insert(key.clone(), value.clone());
                    }
                    count += 1;
                }
            }
        }

        if count > 0 {
            self.save();
        }
        count
    }

    /// Delete documents matching a query
    /// Returns number of documents deleted
    pub fn delete(&mut self, query: &serde_json::Value) -> usize {
        let query_obj: &serde_json::Map<String, serde_json::Value> = match query.as_object() {
            Some(obj) => obj,
            None => return 0,
        };

        let ids_to_delete: Vec<String> = self
            .data
            .iter()
            .filter(|(_, doc): &(&String, &serde_json::Value)| {
                if let Some(doc_obj) = doc.as_object() {
                    query_obj.iter().all(|(key, value): (&String, &serde_json::Value)| {
                        doc_obj.get(key).map_or(false, |v: &serde_json::Value| v == value)
                    })
                } else {
                    false
                }
            })
            .map(|(id, _): (&String, &serde_json::Value)| id.clone())
            .collect();

        let count: usize = ids_to_delete.len();
        for id in ids_to_delete {
            self.data.remove(&id);
        }

        if count > 0 {
            self.save();
        }
        count
    }

    /// Save data to disk
    fn save(&self) {
        let storage: serde_json::Value = serde_json::json!({
            "data": self.data,
            "next_id": self.next_id
        });
        
        if let Ok(json) = serde_json::to_string_pretty(&storage) {
            let _: Result<(), std::io::Error> = fs::write(&self.storage_path, json);
        }
    }

    /// Load data from disk
    fn load(&mut self) {
        if Path::new(&self.storage_path).exists() {
            if let Ok(contents) = fs::read_to_string(&self.storage_path) {
                if let Ok(storage) = serde_json::from_str::<serde_json::Value>(&contents) {
                    if let Some(data_obj) = storage.get("data").and_then(|d: &serde_json::Value| d.as_object()) {
                        for (key, value) in data_obj {
                            self.data.insert(key.clone(), value.clone());
                        }
                    }
                    if let Some(next_id) = storage.get("next_id").and_then(|n: &serde_json::Value| n.as_u64()) {
                        self.next_id = next_id;
                    }
                }
            }
        }
    }
}
