# Clay Studio Backend

Rust backend for Clay Studio using Salvo web framework and SeaORM.

## Development

```bash
# Install dependencies and run
cargo run

# Build for production
cargo build --release
```

## API Endpoints

- `GET /api/health` - Health check
- `POST /api/chat` - Send chat messages
- `GET /api/conversations` - List conversations
- `POST /api/conversations` - Create conversation
- `GET /api/conversations/:id` - Get conversation details
- `PUT /api/conversations/:id` - Update conversation
- `DELETE /api/conversations/:id` - Delete conversation
- `GET /api/conversations/:id/context` - Get conversation context
- `GET /api/projects/:id/context` - Get project context

## Environment Variables

- `DATABASE_URL` - PostgreSQL connection string
- `SERVER_ADDRESS` - Server bind address (default: 127.0.0.1:7680)
- `JWT_SECRET` - Secret key for JWT tokens
- `RUST_LOG` - Logging level (info, debug, trace, warn)