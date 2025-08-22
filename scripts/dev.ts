#!/usr/bin/env bun

import { spawn, spawnSync } from "bun";
import { existsSync } from "fs";
import { join } from "path";

const rootDir = join(import.meta.dir, "..");
const frontendDir = join(rootDir, "frontend");
const backendDir = join(rootDir, "backend");

// Kill processes on port 7680 (backend)
spawnSync({
  cmd: ["lsof", "-ti:7680"],
  stdout: "pipe",
  stderr: "pipe",
})
  .stdout?.toString()
  .trim()
  .split("\n")
  .filter(Boolean)
  .forEach((pid) => {
    if (pid) {
      spawnSync({ cmd: ["kill", "-9", pid] });
      console.log(`  Killed process ${pid} on port 7680`);
    }
  });

// Kill processes on port 7690 (frontend)
spawnSync({
  cmd: ["lsof", "-ti:7690"],
  stdout: "pipe",
  stderr: "pipe",
})
  .stdout?.toString()
  .trim()
  .split("\n")
  .filter(Boolean)
  .forEach((pid) => {
    if (pid) {
      spawnSync({ cmd: ["kill", "-9", pid] });
      console.log(`  Killed process ${pid} on port 7690`);
    }
  });

// Start frontend dev server (silently)
const frontendProcess = spawn({
  cmd: ["bun", "run", "--silent", "dev"],
  cwd: frontendDir,
  stdout: "pipe",
  stderr: "pipe",
  env: {
    ...process.env,
  },
});

// Handle frontend stdout asynchronously
(async () => {
  if (frontendProcess.stdout) {
    for await (const chunk of frontendProcess.stdout) {
      const text = new TextDecoder().decode(chunk);
      // Filter out Vite's startup messages
      if (
        !text.includes("VITE") &&
        !text.includes("$ vite") &&
        !text.includes("ready in") &&
        !text.includes("Local:") &&
        !text.includes("Network:") &&
        !text.includes("press h + enter")
      ) {
        process.stdout.write(chunk);
      }
    }
  }
})();

// Handle frontend stderr
(async () => {
  if (frontendProcess.stderr) {
    for await (const chunk of frontendProcess.stderr) {
      process.stderr.write(chunk);
    }
  }
})();

// Build and start backend
const backendProcess = spawn({
  cmd: ["cargo", "run"],
  cwd: backendDir,
  stdout: "pipe",
  stderr: "pipe",
  env: {
    ...process.env,
    RUST_LOG: "warn",
  },
});

// Handle frontend stdout asynchronously
(async () => {
  if (backendProcess.stdout) {
    for await (const chunk of backendProcess.stdout) {
      process.stderr.write(chunk);
    }
  }
})();

// Handle backend stderr
(async () => {
  if (backendProcess.stderr) {
    for await (const chunk of backendProcess.stderr) {
      const text = new TextDecoder().decode(chunk);
      const trim = text.trim();

      // Filter out Vite's startup messages
      if (trim.startsWith("Running")) {
        console.log(`✨ Clay Studio is running at http://localhost:7690`);
      } else if (!trim.startsWith("Finished")) {
        process.stderr.write(chunk);
      }
    }
  }
})();

// Handle process termination
process.on("SIGINT", () => {
  console.log("\n\n➜ Shutting down development servers...");
  frontendProcess.kill();
  backendProcess.kill();
  process.exit(0);
});

process.on("SIGTERM", () => {
  frontendProcess.kill();
  backendProcess.kill();
  process.exit(0);
});

// Keep the script running
await new Promise(() => {});
