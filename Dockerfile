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
COPY backend/Cargo.toml ./
COPY backend/Cargo.lock* ./
COPY backend/migration ./migration
COPY backend/src ./src
RUN cargo build --release

# Stage 3: Runtime
FROM debian:trixie-slim
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
    rm -rf /var/lib/apt/lists/*

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

# Set ownership of the application directory
RUN chown -R clayuser:clayuser /app

# Set environment variables
ENV RUST_LOG=info
ENV PORT=7680
ENV STATIC_FILES_PATH=/app/frontend/dist
ENV CLIENTS_DIR=/app/.clients
ENV HOME=/app

# Switch to non-root user
USER clayuser

# Declare volume for persistent client data
VOLUME ["/app/.clients"]

# Expose the backend port
EXPOSE 7680

# Run the binary
CMD ["/app/clay-studio-backend"]