#!/bin/bash

# CI Test Runner Script
# This script replicates the GitHub Actions CI workflow locally

echo "🚀 Starting CI Test Runner..."
echo "================================"

# Set environment variables
export CARGO_TERM_COLOR=always

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo is not installed. Please install Rust first."
    echo "Visit: https://rustup.rs/"
    exit 1
fi

echo "✅ Rust/Cargo found"

# Install wasm32-unknown-unknown target if not already installed
echo "📦 Installing wasm32-unknown-unknown target..."
rustup target add wasm32-unknown-unknown

# Install cargo-audit if not already installed
echo "🔍 Installing cargo-audit..."
cargo install cargo-audit --quiet

echo ""
echo "🧪 Running CI Tests..."
echo "======================"

# Step 1: Check Formatting
echo "1️⃣ Checking code formatting..."
if cargo fmt -- --check; then
    echo "✅ Code formatting check passed"
else
    echo "❌ Code formatting check failed"
    echo "Run 'cargo fmt' to fix formatting issues"
    exit 1
fi

echo ""

# Step 2: Run Clippy
echo "2️⃣ Running Clippy lints..."
if cargo clippy --all-targets --all-features -- -D warnings; then
    echo "✅ Clippy checks passed"
else
    echo "❌ Clippy checks failed"
    exit 1
fi

echo ""

# Step 3: Security Audit
echo "3️⃣ Running security audit..."
if cargo audit; then
    echo "✅ Security audit passed"
else
    echo "❌ Security audit found issues"
    exit 1
fi

echo ""

# Step 4: Build Contracts
echo "4️⃣ Building contracts for wasm32-unknown-unknown..."
if cargo build --release --target wasm32-unknown-unknown; then
    echo "✅ Contract build successful"
else
    echo "❌ Contract build failed"
    exit 1
fi

echo ""

# Step 5: Run Tests (Additional step not in CI but important)
echo "5️⃣ Running unit tests..."
if cargo test --workspace; then
    echo "✅ All tests passed"
else
    echo "❌ Some tests failed"
    exit 1
fi

echo ""
echo "🎉 All CI checks passed successfully!"
echo "===================================="

# Additional: Run marketplace approval tests specifically
echo ""
echo "🔍 Running marketplace approval tests specifically..."
echo "===================================================="

if cargo test -p marketplace test_approval --lib; then
    echo "✅ Marketplace approval tests passed"
else
    echo "❌ Marketplace approval tests failed"
    exit 1
fi

echo ""
echo "🎯 All tests completed successfully!"
echo "Your code is ready for deployment."
