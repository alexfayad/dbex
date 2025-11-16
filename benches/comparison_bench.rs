use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dbex::SimpleJSONDB;
use serde_json::json;
use mongodb::bson::Document;

fn comparison_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("comparison");
    
    // Your DB: Insert benchmark
    group.bench_function("dbex_insert_10", |b| {
        let mut db = SimpleJSONDB::new("bench_comparison.json");
        let doc = json!({
            "name": "Test User",
            "age": 30,
            "city": "New York"
        });
        
        b.iter(|| {
            for _ in 0..10 {
                db.insert(black_box(doc.clone()));
            }
        });
    });
    
    // Your DB: Query benchmark
    group.bench_function("dbex_query", |b| {
        let mut db = SimpleJSONDB::new("bench_comparison_query.json");
        
        // Setup - smaller dataset
        for i in 0..100 {
            db.insert(json!({
                "name": format!("User {}", i),
                "age": 20 + (i % 50),
                "city": if i % 2 == 0 { "New York" } else { "San Francisco" }
            }));
        }
        
        let query = json!({"city": "New York"});
        b.iter(|| {
            for _ in 0..10 {
                black_box(db.find(black_box(&query)));
            }
        });
    });
    
    // MongoDB: Insert benchmark (if available)
    group.bench_function("mongodb_insert_10", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        b.iter(|| {
            rt.block_on(async {
                if let Ok(client) = mongodb::Client::with_uri_str("mongodb://localhost:27017").await {
                    let db = client.database("benchmark");
                    let collection = db.collection("test");
                    collection.drop().await.ok();
                    
                    for i in 0..10 {
                        let mut doc = Document::new();
                        doc.insert("name", format!("User {}", i));
                        doc.insert("age", 30i32);
                        doc.insert("city", "New York");
                        collection.insert_one(doc).await.ok();
                    }
                }
            });
        });
    });
    
    // MongoDB: Query benchmark (if available)
    group.bench_function("mongodb_query", |b| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        
        // Setup once - smaller dataset
        rt.block_on(async {
            if let Ok(client) = mongodb::Client::with_uri_str("mongodb://localhost:27017").await {
                let db = client.database("benchmark");
                let collection = db.collection("test");
                collection.drop().await.ok();
                
                for i in 0..100 {
                    let mut doc = Document::new();
                    doc.insert("name", format!("User {}", i));
                    doc.insert("age", (20 + (i % 50)) as i32);
                    doc.insert("city", if i % 2 == 0 { "New York" } else { "San Francisco" });
                    collection.insert_one(doc).await.ok();
                }
            }
        });
        
        b.iter(|| {
            rt.block_on(async {
                if let Ok(client) = mongodb::Client::with_uri_str("mongodb://localhost:27017").await {
                    let db = client.database("benchmark");
                    let collection: mongodb::Collection<Document> = db.collection("test");
                    
                    for _ in 0..10 {
                        let mut filter = Document::new();
                        filter.insert("city", "New York");
                        collection.find_one(filter).await.ok();
                    }
                }
            });
        });
    });
    
    group.finish();
}

criterion_group!(benches, comparison_benchmark);
criterion_main!(benches);
