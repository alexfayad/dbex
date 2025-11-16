#!/bin/bash

# MongoDB Comparison Script
# This script runs benchmarks on both your DB and MongoDB and compares results

echo "=========================================="
echo "Database Performance Comparison"
echo "=========================================="
echo ""

# Check if MongoDB is running
if ! pgrep -x "mongod" > /dev/null; then
    echo "⚠️  MongoDB is not running!"
    echo "   Start it with: mongod --dbpath ./mongo_data"
    echo "   Or install with: brew install mongodb-community (macOS)"
    echo ""
    exit 1
fi

echo "✅ MongoDB is running"
echo ""

# Run your database benchmarks
echo "Running dbex benchmarks..."
cargo bench --bench comparison_bench 2>&1 | grep -A 5 "comparison"

echo ""
echo "=========================================="
echo "MongoDB Benchmarks"
echo "=========================================="
echo ""

# Run MongoDB benchmarks using Rust
echo "Running MongoDB benchmarks..."
cargo run --release --bin mongo_bench 2>&1 || echo "Run: cargo build --release --bin mongo_bench first"

echo ""
echo "=========================================="
echo "Comparison Summary"
echo "=========================================="
echo ""
echo "Check target/criterion/comparison/ for detailed reports"
echo ""

