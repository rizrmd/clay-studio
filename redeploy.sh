#!/bin/bash

# Pull latest changes
echo "📥 Pulling latest changes..."
git pull

# Build new image
echo "🔨 Building Docker image..."
docker build -t ghcr.io/rizrmd/clay-studio:latest .

# Stop existing container
echo "🛑 Stopping existing container..."
docker stop clay-studio || true
docker rm clay-studio || true

# Run new container with volume mount
echo "▶️  Starting new container..."
docker run -d \
  --name clay-studio \
  -p 3000:7680 \
  --restart unless-stopped \
  -v clay-studio-clients:/app/.clients \
  -e DATABASE_URL="postgres://postgres:hltsXdfWOOGNkd32xsMbzp6bgBXPzPCiob6UEH0XL3qSt9OuqvEyhN0o3gZNSjuY@107.155.75.50:5389/clay-studio" \
  -e RUST_ENV=production \
  -e RUST_LOG=info \
  -e STATIC_FILES_PATH=/app/frontend/dist \
  ghcr.io/rizrmd/clay-studio:latest

# Verify the user
echo "🔍 Verifying container user..."
docker exec clay-studio whoami

echo "✅ Deployment complete!"