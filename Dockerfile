# Multi-stage Dockerfile for Clay Studio with optimizations

# Stage 1: Rust dependency caching with cargo-chef
FROM rust:1.89 AS chef
RUN cargo install cargo-chef
WORKDIR /app

# Stage 2: Plan Rust dependencies
FROM chef AS planner
COPY backend/Cargo.toml backend/Cargo.lock* ./
COPY backend/migration ./migration
COPY backend/src ./src
RUN cargo chef prepare --recipe-path recipe.json

# Stage 3: Build Rust dependencies (cached layer)
FROM chef AS rust-cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Stage 4: Build backend
FROM rust:1.89 AS backend-builder
WORKDIR /app
# Copy pre-built dependencies
COPY --from=rust-cacher /app/target target
COPY --from=rust-cacher /usr/local/cargo /usr/local/cargo
# Copy source code
COPY backend/Cargo.toml backend/Cargo.lock* ./
COPY backend/migration ./migration
COPY backend/src ./src
# Build only the changed code (both binaries)
RUN cargo build --release --bin clay-studio-backend && \
    cargo build --release --bin mcp_server

# Stage 5: Build frontend (runs parallel to backend)
FROM oven/bun:1 AS frontend-builder
WORKDIR /app
# Cache dependencies
COPY frontend/package.json frontend/bun.lockb* ./
RUN bun install --frozen-lockfile || bun install
# Build application
COPY frontend/ ./
RUN bun run build

# Stage 6: Runtime (optimized base image)
FROM debian:trixie-slim AS runtime
WORKDIR /app

# Install minimal runtime dependencies including tools needed for Claude CLI setup
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    bash \
    unzip && \
    rm -rf /var/lib/apt/lists/* && \
    apt-get clean

# Create non-root user for security
RUN groupadd -r clayuser && \
    useradd -r -g clayuser -d /app -s /bin/bash clayuser

# Copy the built artifacts from the builder stages
COPY --from=backend-builder /app/target/release/clay-studio-backend /app/clay-studio-backend
COPY --from=backend-builder /app/target/release/mcp_server /app/mcp_server
COPY --from=frontend-builder /app/dist /app/frontend/dist

# Create .clients directory for client data with proper permissions
RUN mkdir -p /app/.clients /app/tmp && \
    chmod 755 /app/.clients /app/tmp

# Pre-install Bun globally for the application to use
ENV BUN_INSTALL=/app/.clients/bun
RUN curl -fsSL https://bun.sh/install | bash && \
    chmod -R 755 /app/.clients

# Set ownership of the entire application directory to clayuser
# This must be done AFTER all files are copied and installed
RUN chown -R clayuser:clayuser /app

# Set environment variables
ENV RUST_LOG=info
ENV PORT=7680
ENV STATIC_FILES_PATH=/app/frontend/dist
ENV CLIENTS_DIR=/app/.clients
ENV HOME=/app

# Remove the entrypoint script that was causing permission issues
# The .clients directory permissions will be handled by the volume mount

# Switch to non-root user
USER clayuser

# Declare volume for persistent client data
VOLUME ["/app/.clients"]

# Expose the backend port
EXPOSE 7680

# Run the binary directly as clayuser
CMD ["/app/clay-studio-backend"]