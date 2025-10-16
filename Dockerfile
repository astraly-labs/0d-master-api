# syntax=docker/dockerfile:1.7

# --- Base with cargo-chef ----------------------------------------------------
FROM lukemathwalker/cargo-chef:latest-rust-1.89.0-slim-bullseye AS cargo-chef
WORKDIR /app

# --- Planner: only what cargo-chef needs ------------------------------------
FROM cargo-chef AS planner
# Speed: don’t send the whole repo if .dockerignore is weak, but ok if it’s strong.
COPY . .
# Cache apt metadata
RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && apt-get install -y curl && rm -rf /var/lib/apt/lists/*
RUN cargo chef prepare --recipe-path recipe.json

# --- Builder: cache registries + target, cache protoc download ---------------
FROM cargo-chef AS builder

# Install build deps (cached)
RUN --mount=type=cache,target=/var/cache/apt \
    apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y \
      libpq-dev pkg-config libssl-dev bash ca-certificates curl wget unzip \
      libclang-dev cmake build-essential \
    && rm -rf /var/lib/apt/lists/*

# protoc (cache the downloaded zip)
RUN --mount=type=cache,target=/var/cache/protoc \
    test -x /usr/local/bin/protoc || ( \
      cd /var/cache/protoc && \
      wget -nc https://github.com/protocolbuffers/protobuf/releases/download/v25.3/protoc-25.3-linux-x86_64.zip && \
      unzip -o protoc-25.3-linux-x86_64.zip && \
      mv bin/protoc /usr/local/bin/ && mv include/* /usr/local/include/ && \
      rm -rf bin include )

# Rust caches
ENV RUSTC_WRAPPER=sccache
RUN cargo install sccache
ENV SCCACHE_DIR=/opt/sccache
RUN mkdir -p ${SCCACHE_DIR}

# Prebuild deps
COPY --from=planner /app/recipe.json recipe.json
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/opt/sccache \
    cargo chef cook --profile release --recipe-path recipe.json

# Build workspace
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/opt/sccache \
    cargo build --locked --release --workspace --exclude tests

# Optional: strip binary to shrink push size
ARG APP_NAME=zerod_bin
RUN strip /app/target/release/${APP_NAME} || true

# --- Final: runtime-only, tiny ------------------------------------------------
FROM gcr.io/distroless/cc-debian12 AS final
# If you need CA certs at runtime:
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
ARG APP_NAME=zerod_bin
COPY --from=builder /app/target/release/${APP_NAME} /bin/server

USER 10001:10001
WORKDIR /app
EXPOSE 3000
ENV RUST_LOG=info
ENTRYPOINT ["/bin/server"]
