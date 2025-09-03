# Agent Instructions for Clay Studio

## Build & Test Commands
```bash
npm run dev          # Run full stack (frontend + backend)
npm run build        # Build production
npm run check        # Type-check and verify compilation
npm run lint         # Frontend linting (cd frontend && npm run lint)
npm run typecheck    # Frontend type checking (cd frontend && npm run typecheck)
npm run sqlx:prepare # Update SQLx query cache before committing backend changes

# Backend (Rust)
cargo test [TESTNAME]  # Run specific test by name
cargo check            # Verify compilation without building
cargo clippy           # Rust linting

# Frontend
cd frontend && npm run dev       # Frontend dev server only
cd frontend && npm run typecheck # TypeScript checking
```

## Critical Code Guidelines
- **API Calls**: ALWAYS use `@/lib/utils/api` - NEVER use fetch/axios directly
- **State Management**: Use valtio, NOT useState (considered code smell)
- **File Size**: Files >1000 lines are code smell, split them up
- **Backend Dev**: Requires `DATABASE_URL=postgres://user:pass@localhost:5432/clay_studio`
- **Import Style**: Use absolute imports with `@/` prefix for frontend src/ files
- **Error Handling**: Wrap async operations in try-catch, use proper error types
- **TypeScript**: Strict typing required, avoid `any` unless absolutely necessary