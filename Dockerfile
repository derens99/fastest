# Build stage — use python:3.12-slim so PyO3 compiles against the same
# Python version used at runtime.
FROM python:3.12-slim AS builder

WORKDIR /app

# Install Rust and build dependencies
RUN apt-get update && apt-get install -y \
    curl \
    pkg-config \
    libssl-dev \
    python3-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust via rustup
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

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
