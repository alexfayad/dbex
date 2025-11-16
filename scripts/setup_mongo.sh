#!/bin/bash

# MongoDB Setup Script
# This script installs and starts MongoDB for benchmarking

set -e

echo "=========================================="
echo "MongoDB Setup for Benchmarking"
echo "=========================================="
echo ""

# Check if MongoDB is already installed
if command -v mongod &> /dev/null; then
    echo "✅ MongoDB is already installed"
    mongod --version
    echo ""
else
    echo "📦 Installing MongoDB..."
    
    # Check if brew is available
    if ! command -v brew &> /dev/null; then
        echo "❌ Homebrew not found. Please install MongoDB manually:"
        echo "   https://www.mongodb.com/docs/manual/installation/"
        exit 1
    fi
    
    # Install MongoDB
    brew tap mongodb/brew
    brew install mongodb-community
    
    echo "✅ MongoDB installed"
    echo ""
fi

# Check if MongoDB is running
if pgrep -x "mongod" > /dev/null; then
    echo "✅ MongoDB is already running"
    echo ""
else
    echo "🚀 Starting MongoDB..."
    
    # Use brew services to start MongoDB (works on macOS)
    brew services start mongodb/brew/mongodb-community
    
    # Wait a bit for MongoDB to start
    sleep 3
    
    if pgrep -x "mongod" > /dev/null; then
        echo "✅ MongoDB started successfully"
        echo ""
    else
        echo "⚠️  MongoDB may still be starting. Wait a few seconds and check with:"
        echo "   pgrep -x mongod"
        echo ""
    fi
fi

echo "=========================================="
echo "MongoDB is ready for benchmarking!"
echo ""
echo "Run benchmarks with:"
echo "  cargo bench --bench comparison_bench"
echo ""
echo "To stop MongoDB:"
echo "  pkill mongod"
echo "=========================================="

