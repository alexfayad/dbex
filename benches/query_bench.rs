use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use dbex::SimpleJSONDB;
use serde_json::json;

fn setup_db_with_data(size: usize) -> SimpleJSONDB {
    let mut db = SimpleJSONDB::new("bench_query_setup.json");
    
    for i in 0..size {
        db.insert(json!({
            "id": i,
            "name": format!("User {}", i),
            "age": 20 + (i % 50),
            "city": if i % 2 == 0 { "New York" } else { "San Francisco" },
            "active": i % 3 == 0
        }));
    }
    
    db
}

fn query_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("query");
    
    // Setup: Insert 1000 documents
    let db = setup_db_with_data(1000);
    
    // Benchmark: Find by ID
    group.bench_function("find_by_id", |b| {
        let db = setup_db_with_data(1000);
        b.iter(|| {
            db.find_by_id(black_box("500"));
        });
    });
    
    // Benchmark: Find all
    group.bench_function("find_all", |b| {
        let db = setup_db_with_data(1000);
        b.iter(|| {
            black_box(db.find_all());
        });
    });
    
    // Benchmark: Query by field
    group.bench_function("find_by_field", |b| {
        let db = setup_db_with_data(1000);
        let query = json!({"city": "New York"});
        b.iter(|| {
            black_box(db.find(black_box(&query)));
        });
    });
    
    // Benchmark: Query with different dataset sizes
    for size in [100, 1000, 10000].iter() {
        group.bench_with_input(
            BenchmarkId::new("find_by_field", size),
            size,
            |b, &size| {
                let db = setup_db_with_data(size);
                let query = json!({"city": "New York"});
                b.iter(|| {
                    black_box(db.find(black_box(&query)));
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, query_benchmark);
criterion_main!(benches);

