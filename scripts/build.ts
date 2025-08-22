#!/usr/bin/env bun

import { spawn } from "bun";
import { join } from "path";

const rootDir = join(import.meta.dir, "..");
const frontendDir = join(rootDir, "frontend");
const backendDir = join(rootDir, "backend");

console.log("🏗️  Building Clay Studio for production...\n");

// Build frontend
console.log("📦 Building frontend...");
const frontendBuild = spawn({
  cmd: ["bun", "run", "build"],
  cwd: frontendDir,
  stdout: "inherit",
  stderr: "inherit",
});

await frontendBuild.exited;

if (frontendBuild.exitCode !== 0) {
  console.error("❌ Frontend build failed!");
  process.exit(1);
}

console.log("✅ Frontend built successfully!\n");

// Build backend
console.log("🦀 Building Rust backend...");
const backendBuild = spawn({
  cmd: ["cargo", "build", "--release"],
  cwd: backendDir,
  stdout: "inherit",
  stderr: "inherit",
});

await backendBuild.exited;

if (backendBuild.exitCode !== 0) {
  console.error("❌ Backend build failed!");
  process.exit(1);
}

console.log("✅ Backend built successfully!");
console.log("\n🎉 Clay Studio built successfully! Run 'bun run prod' to start the production server.");