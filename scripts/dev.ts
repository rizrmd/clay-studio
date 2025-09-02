#!/usr/bin/env bun

import { spawn, spawnSync } from "bun";
import { existsSync } from "fs";
import { join } from "path";

const rootDir = join(import.meta.dir, "..");
const frontendDir = join(rootDir, "frontend");
const backendDir = join(rootDir, "backend");

// Helper function to clean up a specific port with proper waiting
const cleanupPort = (port: number) => {
  const result = spawnSync({
    cmd: ["lsof", "-ti:" + port],
    stdout: "pipe",
    stderr: "pipe",
  });
  
  const pids = result.stdout?.toString()
    .trim()
    .split("\n")
    .filter(Boolean);
    
  if (pids && pids.length > 0) {
    pids.forEach((pid) => {
      if (pid) {
        spawnSync({ cmd: ["kill", "-9", pid] });
        console.log(`  🧹 Cleaned up process ${pid} on port ${port}`);
      }
    });
    // Wait longer to ensure cleanup is complete and port is released
    spawnSync({ cmd: ["sleep", "0.5"] });
    
    // Verify port is actually free
    const verifyResult = spawnSync({
      cmd: ["lsof", "-ti:" + port],
      stdout: "pipe",
      stderr: "pipe",
    });
    
    if (verifyResult.stdout?.toString().trim()) {
      console.log(`  ⚠️  Port ${port} still in use after cleanup, waiting more...`);
      spawnSync({ cmd: ["sleep", "1"] });
    } else {
      console.log(`  ✅ Port ${port} is now free`);
    }
  }
};

// Initial cleanup - kill processes on ports 7680 and 7690
console.log("🧹 Cleaning up existing processes...");
cleanupPort(7680); // backend
cleanupPort(7690); // frontend

// Build MCP server debug binary first
console.log("🔧 Building MCP server debug binary...");
const mcpBuildResult = spawnSync({
  cmd: ["cargo", "build", "--bin", "mcp_server"],
  cwd: backendDir,
  stdout: "pipe",
  stderr: "pipe",
});

if (mcpBuildResult.exitCode === 0) {
  console.log("✅ MCP server debug binary built successfully");
} else {
  console.error("❌ MCP server build failed:", new TextDecoder().decode(mcpBuildResult.stderr));
}

// Build and start backend with watching - ignore target directory and only watch src files
let backendProcess = spawn({
  cmd: ["cargo", "watch", "--ignore", "target/*", "--ignore", "**/.DS_Store", "--ignore", "**/*.tmp", "--delay", "1", "-w", "src", "-w", "Cargo.toml", "-w", "migration", "-x", "run"],
  cwd: backendDir,
  stdout: "pipe",
  stderr: "pipe",
  env: {
    ...process.env,
    RUST_LOG: "warn",
  },
});

// Track backend state
let backendRunning = false;
let frontendStarted = false;
let restartAttempts = 0;
const MAX_RESTART_ATTEMPTS = 5;

// Function to restart backend
const restartBackend = async () => {
  console.log(`🔄 Restarting backend (attempt ${restartAttempts + 1}/${MAX_RESTART_ATTEMPTS})...`);
  
  // Kill existing process
  if (backendProcess) {
    backendProcess.kill();
  }
  
  // Clean up port
  cleanupPort(7680);
  
  // Wait a bit for cleanup
  await new Promise(resolve => setTimeout(resolve, 1000));
  
  // Restart cargo watch
  backendProcess = spawn({
    cmd: ["cargo", "watch", "--ignore", "target/*", "--ignore", "**/.DS_Store", "--ignore", "**/*.tmp", "--delay", "1", "-w", "src", "-w", "Cargo.toml", "-w", "migration", "-x", "run"],
    cwd: backendDir,
    stdout: "pipe",
    stderr: "pipe",
    env: {
      ...process.env,
      RUST_LOG: "warn",
    },
  });
  
  restartAttempts++;
  
  // Re-attach monitoring
  monitorBackend();
};

// Monitor backend output continuously
const monitorBackend = async () => {
  // Monitor backend stdout for the "listening" message
  (async () => {
    if (backendProcess.stdout) {
      for await (const chunk of backendProcess.stdout) {
        const text = new TextDecoder().decode(chunk);
        
        // Output stdout to console
        process.stdout.write(chunk);
        
        if (text.includes("listening") && text.includes("7680")) {
          if (!backendRunning) {
            backendRunning = true;
            restartAttempts = 0; // Reset restart counter on successful start
            console.log("✅ Backend started successfully");
            
            // Start frontend if not already started
            if (!frontendStarted) {
              startFrontend();
            }
            
            // Show the success message
            setTimeout(() => {
              console.log(`✨ Clay Studio is running at http://localhost:7690`);
            }, 100);
          }
        }
      }
    }
  })();
  
  // Monitor backend stderr for compilation status
  (async () => {
    if (backendProcess.stderr) {
      let compilationInProgress = false;
      let hasErrors = false;
      
      for await (const chunk of backendProcess.stderr) {
        const text = new TextDecoder().decode(chunk);
        
        // Always output to see what's happening
        process.stderr.write(chunk);
        
        // Detect compilation start
        if (text.includes("Compiling")) {
          compilationInProgress = true;
          hasErrors = false;
          if (backendRunning) {
            console.log("🔄 Recompiling backend...");
            backendRunning = false;
            // Clean up port immediately when recompilation starts
            cleanupPort(7680);
          } else {
            console.log("🔧 Compiling backend...");
          }
        }
        
        // Detect compilation errors
        if (text.includes("error[E") || text.includes("error: could not compile")) {
          hasErrors = true;
          backendRunning = false;
          console.error("❌ Backend compilation failed - waiting for fixes...");
        }
        
        // Detect successful compilation (cargo watch will run the binary after successful compile)
        if (compilationInProgress && text.includes("Finished") && !hasErrors) {
          compilationInProgress = false;
          console.log("✅ Backend compiled successfully - starting...");
          
          // Reset backend running state - let the "listening" detection handle startup
          backendRunning = false;
          
          // Also rebuild MCP server when backend recompiles
          console.log("🔧 Rebuilding MCP server debug binary...");
          try {
            const mcpRebuild = spawn({
              cmd: ["cargo", "build", "--bin", "mcp_server"],
              cwd: backendDir,
              stdout: "pipe",
              stderr: "pipe",
            });
            
            // Check if spawn was successful
            if (mcpRebuild && mcpRebuild.exited) {
              // Don't await - let it rebuild in background
              mcpRebuild.exited.then((exitCode) => {
                if (exitCode === 0) {
                  console.log("✅ MCP server debug binary rebuilt successfully");
                } else {
                  console.error("❌ MCP server rebuild failed with exit code:", exitCode);
                }
              }).catch((error) => {
                console.error("❌ MCP server rebuild error:", error);
              });
            } else {
              console.error("❌ Failed to spawn MCP server rebuild process");
            }
          } catch (error) {
            console.error("❌ Error starting MCP server rebuild:", error);
          }
        }
        
        // Detect process crashes and kills at runtime
        if (text.includes("thread 'main' panicked") || text.includes("error: process didn't exit successfully") || text.includes("Address already in use") || text.includes("Killed: 9")) {
          backendRunning = false;
          if (text.includes("Killed: 9")) {
            console.log("🔄 Backend process restarted by cargo watch");
          } else {
            console.error("💥 Backend crashed - cleaning up and restarting...");
            
            // Clean up port 7680 immediately when we detect a crash
            cleanupPort(7680);
            
            // Restart backend if we haven't exceeded max attempts
            if (restartAttempts < MAX_RESTART_ATTEMPTS) {
              setTimeout(() => {
                restartBackend();
              }, 2000); // Wait 2 seconds before restart
            } else {
              console.error(`❌ Backend crashed ${MAX_RESTART_ATTEMPTS} times. Please check for issues.`);
              process.exit(1);
            }
          }
        }
      }
    }
  })();
  
  // Handle if the backend process exits unexpectedly
  backendProcess.exited.then((exitCode) => {
    console.error(`Backend process exited with code ${exitCode}`);
    process.exit(1);
  });
};

// Start monitoring backend
monitorBackend();

// Function to start frontend
let frontendProcess: any = null;

const startFrontend = () => {
  if (frontendStarted) return;
  frontendStarted = true;
  
  console.log("🚀 Starting frontend dev server...");
  
  // Start frontend dev server (silently)
  frontendProcess = spawn({
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
};


// Handle process termination
process.on("SIGINT", () => {
  console.log("\n\n➜ Shutting down development servers...");
  if (frontendProcess) frontendProcess.kill();
  backendProcess.kill();
  process.exit(0);
});

process.on("SIGTERM", () => {
  if (frontendProcess) frontendProcess.kill();
  backendProcess.kill();
  process.exit(0);
});

// Keep the script running
await new Promise(() => {});
