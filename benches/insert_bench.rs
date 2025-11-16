use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use dbex::SimpleJSONDB;
use serde_json::json;

fn insert_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");
    
    // Benchmark single insert
    group.bench_function("single_insert", |b| {
        let mut db = SimpleJSONDB::new("bench_insert.json");
        let doc = json!({
            "name": "Test User",
            "age": 30,
            "city": "New York",
            "tags": ["developer", "rust"],
            "active": true
        });
        
        b.iter(|| {
            db.insert(black_box(doc.clone()));
        });
    });
    
    // Benchmark batch inserts (10, 100, 1000)
    for size in [10, 100, 1000].iter() {
        group.bench_with_input(
            BenchmarkId::new("batch_insert", size),
            size,
            |b, &size| {
                let mut db = SimpleJSONDB::new("bench_batch.json");
                let doc = json!({
                    "name": "Test User",
                    "age": 30,
                    "city": "New York"
                });
                
                b.iter(|| {
                    for _ in 0..size {
                        db.insert(black_box(doc.clone()));
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(benches, insert_benchmark);
criterion_main!(benches);

