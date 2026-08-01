FROM rust:1.95.0-bookworm@sha256:6258907abe69656e41cd992e0b705cdcfabcbbe3db374f92ed2d47121282d4a1

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get install --yes --no-install-recommends python3 git ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /workspace/AetherOS
