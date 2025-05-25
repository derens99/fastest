# Base image for building the fastest project
FROM python:3.11-slim AS build

# Install Rust and maturin
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential curl \
    && curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    && /root/.cargo/bin/cargo install --locked maturin \
    && rm -rf /var/lib/apt/lists/*

ENV PATH="/root/.cargo/bin:${PATH}"
WORKDIR /usr/src/fastest

COPY . .
RUN maturin build --release --manylinux off

FROM python:3.11-slim

COPY --from=build /usr/src/fastest/target/wheels/*.whl /tmp/
RUN pip install /tmp/fastest-*.whl && rm -rf /tmp/fastest-*.whl

ENTRYPOINT ["fastest"]
CMD ["--help"]
