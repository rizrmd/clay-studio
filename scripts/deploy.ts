#!/usr/bin/env bun

import { $ } from "bun";

const GITHUB_USER = "rizrmd";
const IMAGE_NAME = "clay-studio";
const REGISTRY = "ghcr.io";
const TAG = "latest";

const imageUrl = `${REGISTRY}/${GITHUB_USER}/${IMAGE_NAME}:${TAG}`;

console.log("🚀 Deploying application...");
console.log(`📦 Image: ${imageUrl}`);

try {
  // Build the image locally instead of pulling from registry
  console.log("🔨 Building Docker image...");
  await $`docker build -t ${imageUrl} .`;

  // Stop and remove existing container if it exists
  console.log("🛑 Stopping existing container...");
  await $`docker stop clay-studio || true`;
  await $`docker rm clay-studio || true`;

  // Run the new container with persistent volume for .clients data
  console.log("▶️  Starting new container...");
  await $`docker run -d --name clay-studio -p 3000:7680 --restart unless-stopped -v clay-studio-clients:/app/.clients -e DATABASE_URL=postgres://postgres:hltsXdfWOOGNkd32xsMbzp6bgBXPzPCiob6UEH0XL3qSt9OuqvEyhN0o3gZNSjuY@107.155.75.50:5389/clay-studio -e RUST_ENV=production -e RUST_LOG=debug -e STATIC_FILES_PATH=/app/frontend/dist ${imageUrl}`;

  console.log("✅ Successfully deployed application!");
  console.log("🌐 Application is running on http://localhost:3000");
} catch (error) {
  console.error("❌ Failed to deploy application:", error);
  process.exit(1);
}