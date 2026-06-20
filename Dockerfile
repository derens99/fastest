# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release --bin fastest

# Runtime stage
FROM python:3.11-slim

# Install Python dependencies that might be needed for testing
RUN apt-get update && apt-get install -y curl ca-certificates \
    && curl -LsSf https://astral.sh/uv/install.sh | sh \
    && /root/.local/bin/uv pip install --system pytest pytest-asyncio coverage \
    && rm -rf /var/lib/apt/lists/*

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
