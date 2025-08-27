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
# Build only the changed code
RUN cargo build --release

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
COPY --from=frontend-builder /app/dist /app/frontend/dist

# Create .clients directory for client data with proper permissions
RUN mkdir -p /app/.clients /app/tmp && \
    chown -R clayuser:clayuser /app/.clients /app/tmp && \
    chmod 755 /app/.clients /app/tmp

# Pre-install Bun globally for the application to use
ENV BUN_INSTALL=/app/.clients/bun
RUN curl -fsSL https://bun.sh/install | bash && \
    chown -R clayuser:clayuser /app/.clients && \
    chmod -R 755 /app/.clients

# Set ownership of the application directory
RUN chown -R clayuser:clayuser /app

# Set environment variables
ENV RUST_LOG=info
ENV PORT=7680
ENV STATIC_FILES_PATH=/app/frontend/dist
ENV CLIENTS_DIR=/app/.clients
ENV HOME=/app

# Create entrypoint script to fix permissions on mounted volume
RUN echo '#!/bin/bash\n\
# Fix permissions on mounted volume if it exists\n\
if [ -d "/app/.clients" ]; then\n\
    sudo chown -R clayuser:clayuser /app/.clients 2>/dev/null || true\n\
    sudo chmod -R 755 /app/.clients 2>/dev/null || true\n\
fi\n\
exec "$@"' > /entrypoint.sh && chmod +x /entrypoint.sh

# Install sudo for the entrypoint script
RUN apt-get update && apt-get install -y --no-install-recommends sudo && \
    echo "clayuser ALL=(ALL) NOPASSWD: /bin/chown, /bin/chmod" >> /etc/sudoers && \
    rm -rf /var/lib/apt/lists/*

# Switch to non-root user
USER clayuser

# Declare volume for persistent client data
VOLUME ["/app/.clients"]

# Expose the backend port
EXPOSE 7680

# Use entrypoint to fix permissions, then run the binary
ENTRYPOINT ["/entrypoint.sh"]
CMD ["/app/clay-studio-backend"]