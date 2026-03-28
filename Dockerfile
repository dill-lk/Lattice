# Lattice Blockchain Node
FROM rust:1.75-slim as builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY bins ./bins

# Build release binary
RUN cargo build --release --bin lattice-node

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 lattice

# Copy binary from builder
COPY --from=builder /app/target/release/lattice-node /usr/local/bin/

# Create data directory
RUN mkdir -p /data && chown lattice:lattice /data

# Switch to non-root user
USER lattice

# Set working directory
WORKDIR /data

# Expose ports
EXPOSE 8545 30333

# Set entrypoint
ENTRYPOINT ["lattice-node"]

# Default command
CMD ["--config", "/data/config/node.toml"]
