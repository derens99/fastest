# Build stage
FROM rust:slim AS builder

WORKDIR /app

# Install build dependencies (OpenSSL for git2, Python for PyO3)
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    python3-dev \
    python3 \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release --bin fastest

# Runtime stage
FROM python:3.12-slim

# Install Python dependencies that might be needed for testing
RUN pip install --no-cache-dir pytest pytest-asyncio

# Copy the binary from builder
COPY --from=builder /app/target/release/fastest /usr/local/bin/fastest

# Create a non-root user
RUN useradd -m -u 1000 testrunner && \
    mkdir -p /workspace && \
    chown -R testrunner:testrunner /workspace

USER testrunner
WORKDIR /workspace

# Set entrypoint
ENTRYPOINT ["fastest"]
CMD ["--help"]
