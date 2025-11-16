use dbex::SimpleJSONDB;
use serde_json::json;

fn main() {
    println!("Testing Database with Dict-Style Objects\n");
    println!("==========================================\n");

    let mut db = SimpleJSONDB::new("test_db.json");

    // Test 1: Insert documents
    println!("Test 1: Insert documents");
    let id1 = db.insert(json!({
        "name": "Alice",
        "age": 30,
        "city": "New York",
        "tags": ["developer", "rust"]
    }));
    println!("  ✓ Inserted document 1, ID: {}", id1);

    let id2 = db.insert(json!({
        "name": "Bob",
        "age": 25,
        "city": "San Francisco",
        "active": true
    }));
    println!("  ✓ Inserted document 2, ID: {}", id2);

    let id3 = db.insert(json!({
        "name": "Charlie",
        "age": 35,
        "city": "New York",
        "metadata": {
            "department": "engineering",
            "level": "senior"
        }
    }));
    println!("  ✓ Inserted document 3, ID: {}", id3);
    println!();

    // Test 2: Find by ID
    println!("Test 2: Find by ID");
    if let Some(doc) = db.find_by_id(&id1) {
        println!("  ✓ Found document by ID: {}", serde_json::to_string_pretty(&doc).unwrap());
    } else {
        println!("  ✗ Failed to find document by ID");
    }
    println!();

    // Test 3: Find all
    println!("Test 3: Find all documents");
    let all = db.find_all();
    println!("  ✓ Found {} documents", all.len());
    for (i, doc) in all.iter().enumerate() {
        println!("  Document {}: {}", i + 1, serde_json::to_string(doc).unwrap());
    }
    println!();

    // Test 4: Query by field
    println!("Test 4: Query by field (city = 'New York')");
    let results = db.find(&json!({"city": "New York"}));
    println!("  ✓ Found {} documents in New York", results.len());
    for doc in &results {
        println!("    {}", serde_json::to_string(doc).unwrap());
    }
    println!();

    // Test 5: Update
    println!("Test 5: Update document");
    let updated = db.update(&json!({"name": "Bob"}), &json!({"age": 26, "status": "updated"}));
    println!("  ✓ Updated {} document(s)", updated);
    if let Some(doc) = db.find_by_id(&id2) {
        println!("  Updated document: {}", serde_json::to_string(&doc).unwrap());
    }
    println!();

    // Test 6: Delete
    println!("Test 6: Delete document");
    let deleted = db.delete(&json!({"name": "Charlie"}));
    println!("  ✓ Deleted {} document(s)", deleted);
    let remaining = db.find_all();
    println!("  Remaining documents: {}", remaining.len());
    println!();

    // Test 7: Complex nested object
    println!("Test 7: Insert complex nested object");
    let id4 = db.insert(json!({
        "user": {
            "profile": {
                "name": "David",
                "settings": {
                    "theme": "dark",
                    "notifications": true
                }
            }
        },
        "posts": [1, 2, 3],
        "created_at": "2024-01-01"
    }));
    println!("  ✓ Inserted nested document, ID: {}", id4);
    if let Some(doc) = db.find_by_id(&id4) {
        println!("  Document: {}", serde_json::to_string_pretty(&doc).unwrap());
    }
    println!();

    println!("==========================================");
    println!("All tests completed!");
}
