# Production Dockerfile for Clay Studio
# Assumes the application has been built with 'bun run build' in CI/CD
# This creates a minimal runtime container

FROM debian:trixie-slim

# Install minimal runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    libpq5 && \
    rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy the pre-built Rust binary from CI/CD build
COPY backend/target/release/clay-studio-backend /app/clay-studio-backend

# Copy the pre-built frontend dist
COPY frontend/dist /app/frontend/dist

# Set environment variables
ENV RUST_LOG=info
ENV PORT=7680

# Expose the backend port
EXPOSE 7680

# Run the binary
CMD ["/app/clay-studio-backend"]