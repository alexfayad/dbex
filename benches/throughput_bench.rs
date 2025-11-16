use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dbex::DBex;
use serde_json::json;
use std::time::Instant;

fn throughput_benchmark(c: &mut Criterion) {
    let mut group = c.benchmark_group("throughput");
    
    // Measure insert throughput (ops/sec)
    group.bench_function("insert_throughput", |b| {
        let mut db = DBex::new("bench_throughput.json");
        let doc = json!({
            "name": "Throughput Test",
            "age": 30,
            "data": vec![0u8; 100] // 100 bytes of data
        });
        
        b.iter_custom(|iters| {
            let start = Instant::now();
            for _ in 0..iters {
                db.insert(black_box(doc.clone()));
            }
            start.elapsed()
        });
    });
    
    // Measure query throughput
    group.bench_function("query_throughput", |b| {
        let mut db = DBex::new("bench_query_throughput.json");
        
        // Pre-populate with 1000 docs
        for i in 0..1000 {
            db.insert(json!({
                "id": i,
                "value": i * 2
            }));
        }
        
        b.iter_custom(|iters| {
            let start = Instant::now();
            for i in 0..iters {
                let id = format!("{}", i % 1000);
                black_box(db.find_by_id(&id));
            }
            start.elapsed()
        });
    });
    
    group.finish();
}

criterion_group!(benches, throughput_benchmark);
criterion_main!(benches);

