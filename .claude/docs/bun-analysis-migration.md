# Analysis Runtime Migration: QuickJS → Bun

## Overview

Successfully migrated analysis script execution from embedded QuickJS runtime to Bun subprocess execution.

## Changes Made

### 1. New Bun Runtime Module (`bun_runtime.rs`)

Created a new runtime wrapper that:
- Spawns Bun processes to execute analysis scripts
- Manages `.clients/{project_id}/analysis/` directories for each project
- Creates execution wrappers with context injection
- Validates scripts using Bun's TypeScript compiler
- Handles timeouts and result parsing

**Key Features:**
- Full ES2020+ support (no syntax stripping needed)
- Native async/await
- NPM package ecosystem access
- Better error messages with stack traces
- 10MB result size limit with JSON validation

### 2. Updated Analysis Sandbox (`sandbox.rs`)

Simplified from complex QuickJS integration to:
- Initialize BunRuntime with `.clients` directory
- Pass script, parameters, and context to Bun
- Handle validation through Bun's compiler

**Removed:**
- QuickJS facade and runtime management
- Complex JS-to-Rust value conversion
- Mock context API injection
- MCP bridge setup code

### 3. Updated Analysis Service (`service.rs`)

Changed from:
- QuickJS embedded execution
- Syntax stripping (export/async removal)
- Blocking spawn tasks

To:
- Bun subprocess execution
- Full TypeScript/ES6 support
- Direct datasource metadata passing

### 4. TypeScript Type Definitions (`analysis-runtime-types.d.ts`)

Created comprehensive type definitions for:
- `AnalysisContext` - Runtime context API
- `FileAPI` - File operations interface
- `QueryResult` - Database query results
- `Analysis` interface - Script structure

### 5. Dependency Changes (`Cargo.toml`)

**Removed:**
- `quickjs_runtime = "0.10"`

**No new Rust dependencies** - Uses system/bundled Bun

### 6. Directory Structure

```
.clients/{project_id}/
  └── analysis/
      ├── package.json        # Per-project dependencies
      ├── node_modules/       # Installed packages
      ├── scripts/            # Saved analysis scripts
      └── temp/               # Temporary execution files
```

##Benefits

### Performance
- **Faster execution**: Bun is highly optimized
- **No syntax stripping**: Execute code as-written
- **Better parallelization**: Each analysis runs in separate process

### Developer Experience
- **Full TypeScript support**: Types, interfaces, generics
- **NPM ecosystem**: Use packages like `csv-parse`, `duckdb`
- **Real console.log**: Output visible in logs
- **Better errors**: Full stack traces with line numbers

### Maintainability
- **Less Rust code**: Removed complex FFI bindings
- **Clearer separation**: JavaScript stays in JavaScript
- **Easier debugging**: Can test scripts with `bun run` directly
- **Type safety**: TypeScript definitions prevent API misuse

## Migration Path for Existing Scripts

Old QuickJS format (with limitations):
```javascript
// Had to avoid async/await, limited APIs
const main = {
  run: (ctx, params) => {
    return { result: "data" };
  }
};
```

New Bun format (full features):
```typescript
export default {
  async run(ctx, parameters) {
    // Full async/await support
    const result = await ctx.query('SELECT * FROM users');

    // Use NPM packages
    const parser = require('csv-parse/sync');

    // Access all context APIs
    ctx.log('Processing...');
    const files = await ctx.files.list();

    return { result };
  }
}
```

## Remaining Work

### High Priority
1. **Implement ctx.query()** - Connect to DuckDB from Bun
2. **Implement ctx.files** - Connect to file storage API
3. **Add datasource connectors** - Query external databases from Bun

### Medium Priority
4. **Streaming results** - Handle > 10MB datasets
5. **Progress tracking** - Report execution progress
6. **Install dependencies automatically** - Run `bun install` on first use

### Low Priority
7. **Script caching** - Cache compiled scripts
8. **Sandbox security** - Limit file system/network access
9. **Resource limits** - CPU/memory quotas per analysis

## Testing

To test the new runtime:

```bash
# Create a test analysis
cd .clients/test-project-id/analysis

# Write test script
cat > temp/test.ts << 'EOF'
export default {
  async run(ctx: any, params: any) {
    ctx.log('Hello from Bun!');
    return { message: 'Success', params };
  }
}
EOF

# Run directly with Bun
bun run temp/test.ts
```

## Compilation Status

✅ All compilation errors resolved
✅ QuickJS dependency removed
✅ Updated all instantiation sites
✅ Fixed type mismatches in analysis_manager.rs

## Next Steps

1. Implement real ctx.query() using DuckDB Node.js binding
2. Implement real ctx.files using HTTP requests back to backend
3. Add integration tests for Bun execution
4. Update frontend to use TypeScript type definitions
5. Document analysis script development guide
