# Multi-stage Dockerfile for Clay Studio

# Stage 1: Build frontend
FROM oven/bun:1 AS frontend-builder
WORKDIR /app
COPY frontend/package.json frontend/bun.lockb* ./
RUN bun install --frozen-lockfile || bun install
COPY frontend/ ./
RUN bun run build

# Stage 2: Build backend
FROM rust:1.89 AS backend-builder
WORKDIR /app
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/migration ./migration
COPY backend/src ./src
RUN cargo build --release

# Stage 3: Runtime
FROM debian:trixie-slim
WORKDIR /app

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 && \
    rm -rf /var/lib/apt/lists/*

# Copy the built artifacts from the builder stages
COPY --from=backend-builder /app/target/release/clay-studio-backend /app/clay-studio-backend
COPY --from=frontend-builder /app/dist /app/frontend/dist

# Set environment variables
ENV RUST_LOG=info
ENV PORT=7680

# Expose the backend port
EXPOSE 7680

# Run the binary
CMD ["/app/clay-studio-backend"]