#!/bin/bash

echo "🔧 Emergency production fix for Clay Studio"
echo "==========================================="

# Pull latest code
echo "📥 Pulling latest changes..."
git pull origin main

# Build the new image with proper user setup
echo "🔨 Building new Docker image with clayuser..."
docker build -t ghcr.io/rizrmd/clay-studio:latest . || exit 1

# Verify clayuser exists in the new image
echo "🔍 Verifying clayuser in new image..."
docker run --rm ghcr.io/rizrmd/clay-studio:latest whoami

# Stop and remove old container
echo "🛑 Stopping old container..."
docker stop clay-studio || true
docker rm clay-studio || true

# Run new container with proper user
echo "▶️  Starting new container with clayuser..."
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

# Wait for container to start
sleep 3

# Verify the user in running container
echo "✅ Verifying user in running container..."
docker exec clay-studio whoami

echo "✅ Deployment complete!"
echo "Container should now be running as 'clayuser', not root"