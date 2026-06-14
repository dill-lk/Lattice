# Lattice Unified Runtime
FROM rust:1.75-slim as builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY bins ./bins
COPY tests ./tests

# Build the official unified executable only
RUN cargo build --release --bin lattice

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -m -u 1000 lattice

COPY --from=builder /app/target/release/lattice /usr/local/bin/

RUN mkdir -p /data && chown lattice:lattice /data

USER lattice
WORKDIR /data

EXPOSE 8545 30303

ENTRYPOINT ["lattice"]
CMD ["--node"]
