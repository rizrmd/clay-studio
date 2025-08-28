#!/bin/bash

echo "🚨 CRITICAL FIX: Rebuilding container to run as clayuser (not root)"
echo "================================================================"

# Stop the current container
echo "1️⃣ Stopping current container running as root..."
docker stop clay-studio
docker rm clay-studio

# Remove old image to force rebuild
echo "2️⃣ Removing old image..."
docker rmi ghcr.io/rizrmd/clay-studio:latest || true

# Pull latest code
echo "3️⃣ Getting latest code..."
git pull origin main

# Build new image with clayuser
echo "4️⃣ Building new Docker image with clayuser..."
docker build --no-cache -t ghcr.io/rizrmd/clay-studio:latest .

# Verify clayuser exists in the image
echo "5️⃣ Verifying clayuser in new image..."
docker run --rm ghcr.io/rizrmd/clay-studio:latest whoami

# Run new container
echo "6️⃣ Starting new container..."
docker run -d \
  --name clay-studio \
  -p 3000:7680 \
  --restart unless-stopped \
  -v clay-studio-clients:/app/.clients \
  -e DATABASE_URL="postgres://postgres:hltsXdfWOOGNkd32xsMbzp6bgBXPzPCiob6UEH0XL3qSt9OuqvEyhN0o3gZNSjuY@107.155.75.50:5389/clay-studio" \
  -e RUST_ENV=production \
  -e RUST_LOG=info \
  ghcr.io/rizrmd/clay-studio:latest

# Wait for startup
sleep 5

# Check the logs
echo "7️⃣ Checking logs for user..."
docker logs clay-studio 2>&1 | grep "Server running as user"

echo ""
echo "✅ If you see 'clayuser' above, the fix is successful!"
echo "❌ If you still see 'root', there's an issue with the Docker build"