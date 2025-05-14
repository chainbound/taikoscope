# Reference guide: https://depot.dev/blog/rust-dockerfile-best-practices

# Run with Debian, libclang has issues with Alpine.
# Make sure to update rust-toolchain.toml when updating the base image,
# and vice versa.
FROM rust:1.86.0-bookworm AS base

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    libssl-dev \
    pkg-config \
    clang \
    llvm-19 \
    libclang-19-dev \
    cmake

RUN cargo install sccache --locked
RUN cargo install cargo-chef --locked

ENV RUSTC_WRAPPER=sccache SCCACHE_DIR=/sccache

FROM base AS planner

WORKDIR /app

COPY . .

RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef prepare --recipe-path recipe.json

FROM base AS builder

WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json

# Add architecture as a build arg
ARG TARGETARCH

RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN --mount=type=cache,target=$SCCACHE_DIR,sharing=locked \
    if [ "$TARGETARCH" = "arm64" ]; then \
    echo "Building for arm64 with JEMALLOC_SYS_WITH_LG_PAGE=16"; \
    # Force jemalloc to use 64 KiB pages on ARM
    # https://github.com/paradigmxyz/reth/pull/7123
    JEMALLOC_SYS_WITH_LG_PAGE=16 cargo build --profile release; \
    else \
    echo "Building for $TARGETARCH"; \
    cargo build --profile release; \
    fi

FROM debian:bookworm-slim AS runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/taikoscope taikoscope

# Add taikoscope user
RUN chmod +x taikoscope && \
    groupadd -r taikoscope && \
    useradd -r -g taikoscope taikoscope

USER taikoscope

ENTRYPOINT ["./taikoscope"]

