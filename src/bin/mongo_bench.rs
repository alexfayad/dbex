use mongodb::{Client, bson};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("MongoDB Benchmark");
    println!("==================");
    println!();

    let client = Client::with_uri_str("mongodb://localhost:27017").await?;
    let db = client.database("benchmark");
    let collection = db.collection("test");
    
    // Clear collection
    collection.drop().await?;
    println!("✅ Connected to MongoDB");
    println!();

    // Insert benchmark
    println!("Insert Benchmark (1000 documents)...");
    let start = Instant::now();
    for i in 0..1000 {
        let doc = bson::doc! {
            "name": format!("User {}", i),
            "age": 20 + (i % 50),
            "city": if i % 2 == 0 { "New York" } else { "San Francisco" },
            "active": i % 3 == 0
        };
        collection.insert_one(doc).await?;
    }
    let elapsed = start.elapsed();
    let ms_per_insert = elapsed.as_secs_f64() * 1000.0 / 1000.0;
    let inserts_per_sec = 1000.0 / (elapsed.as_secs_f64());
    
    println!("  Time: {:.2}ms total", elapsed.as_secs_f64() * 1000.0);
    println!("  Per insert: {:.4}ms", ms_per_insert);
    println!("  Throughput: {:.0} inserts/sec", inserts_per_sec);
    println!();

    // Query benchmark
    println!("Query Benchmark (1000 queries by city)...");
    let start = Instant::now();
    for _ in 0..1000 {
        let filter = bson::doc! { "city": "New York" };
        let _ = collection.find_one(filter).await?;
    }
    let elapsed = start.elapsed();
    let ms_per_query = elapsed.as_secs_f64() * 1000.0 / 1000.0;
    let queries_per_sec = 1000.0 / (elapsed.as_secs_f64());
    
    println!("  Time: {:.2}ms total", elapsed.as_secs_f64() * 1000.0);
    println!("  Per query: {:.4}ms", ms_per_query);
    println!("  Throughput: {:.0} queries/sec", queries_per_sec);
    println!();

    println!("==========================================");
    println!("Compare these numbers with your dbex benchmarks!");
    println!("Run: cargo bench --bench comparison_bench");
    println!();

    Ok(())
}

