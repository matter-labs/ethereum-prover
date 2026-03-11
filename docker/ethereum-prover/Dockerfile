FROM nvidia/cuda:12.9.1-cudnn-devel-ubuntu24.04 AS chef

COPY rust-toolchain rust-toolchain

ARG DEBIAN_FRONTEND=noninteractive

# These build args are optional. CI can opt into sccache-backed builds,
# while local `docker build` stays usable without Matter Labs infra access.
ARG SCCACHE_GCS_BUCKET
ARG SCCACHE_GCS_SERVICE_ACCOUNT
ARG SCCACHE_GCS_RW_MODE
ARG RUSTC_WRAPPER

ARG CUDA_ARCH=89;100
ENV CUDAARCHS=${CUDA_ARCH}

RUN apt-get update && apt-get install -y curl clang openssl libssl-dev gcc g++ git \
    pkg-config build-essential libclang-dev cmake sccache && \
    rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:${PATH} \
    RUST_MIN_STACK=267108864

RUN curl https://sh.rustup.rs -sSf | bash -s -- -y

# We keep the dependency build separate so Docker layer caching can reuse it
# across source-only changes.
RUN cargo install cargo-chef --version 0.1.72

WORKDIR /usr/src/ethereum-prover

FROM chef AS planner
COPY . .
RUN cargo chef prepare --bin ethereum_prover --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /usr/src/ethereum-prover/recipe.json recipe.json
RUN cargo chef cook -p ethereum_prover --release --recipe-path recipe.json

COPY . .
RUN cargo build -p ethereum_prover --release

# The runtime bundle intentionally preserves the repo-like layout.
# The shipped configs still point to ../artifacts/app.bin, so we keep that
# contract instead of maintaining a container-only fork of the config files.
RUN mkdir -p /out/ethereum_prover/configs /out/artifacts && \
    cp /usr/src/ethereum-prover/target/release/ethereum_prover /out/ethereum_prover/ && \
    cp /usr/src/ethereum-prover/ethereum_prover/.env.example /out/ethereum_prover/ && \
    cp /usr/src/ethereum-prover/ethereum_prover/configs/*.yaml /out/ethereum_prover/configs/ && \
    cp /usr/src/ethereum-prover/artifacts/app.bin /out/artifacts/ && \
    cp /usr/src/ethereum-prover/artifacts/recursion_unified_setup.bin /out/artifacts/ && \
    cp /usr/src/ethereum-prover/artifacts/recursion_unified_layouts.bin /out/artifacts/

FROM nvidia/cuda:12.2.0-runtime-ubuntu22.04

ARG DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y ca-certificates curl libssl3 && \
    rm -rf /var/lib/apt/lists/*

ENV NVIDIA_VISIBLE_DEVICES=all \
    NVIDIA_DRIVER_CAPABILITIES=compute,utility \
    RUST_MIN_STACK=267108864

WORKDIR /opt/ethereum-prover/ethereum_prover

COPY --from=builder /out /opt/ethereum-prover/

ENTRYPOINT ["./ethereum_prover"]
