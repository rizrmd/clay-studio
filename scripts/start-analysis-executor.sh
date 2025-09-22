#!/bin/bash

# Analysis Executor Development Startup Script

set -e

echo "🚀 Starting Clay Studio Analysis Executor"

# Check if DATABASE_URL is set
if [ -z "$DATABASE_URL" ]; then
    echo "❌ DATABASE_URL not set. Using default..."
    export DATABASE_URL="postgres://localhost:5432/clay_studio"
fi

# Check if migrations have been run
echo "📊 Checking database connection..."
if ! psql "$DATABASE_URL" -c "SELECT 1;" > /dev/null 2>&1; then
    echo "❌ Cannot connect to database. Please check DATABASE_URL: $DATABASE_URL"
    exit 1
fi

# Check if analysis tables exist
echo "🔍 Checking if analysis tables exist..."
if ! psql "$DATABASE_URL" -c "SELECT 1 FROM analyses LIMIT 1;" > /dev/null 2>&1; then
    echo "⚠️  Analysis tables not found. Running migrations..."
    cd backend
    sqlx migrate run --source ./migrations
    cd ..
fi

# Set default port if not specified
if [ -z "$ANALYSIS_EXECUTOR_PORT" ]; then
    export ANALYSIS_EXECUTOR_PORT=8002
fi

echo "🔧 Configuration:"
echo "   Database: $DATABASE_URL"
echo "   Port: $ANALYSIS_EXECUTOR_PORT"
echo "   Log Level: ${RUST_LOG:-info}"

# Start the analysis executor
echo "🎯 Starting Analysis Executor on port $ANALYSIS_EXECUTOR_PORT..."
cd backend

if [ "$1" = "--release" ]; then
    echo "📦 Building release binary..."
    cargo build --release --bin analysis_executor
    exec ./target/release/analysis_executor
else
    echo "🏗️  Running in development mode..."
    exec cargo run --bin analysis_executor
fi