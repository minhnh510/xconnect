# syntax=docker/dockerfile:1

FROM rust:1.76-bookworm AS builder
WORKDIR /workspace

COPY Cargo.toml rust-toolchain.toml ./
COPY crates ./crates
COPY services/control-plane ./services/control-plane

RUN cargo build --release -p control-plane

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
  && apt-get install -y --no-install-recommends ca-certificates \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /workspace/target/release/control-plane /usr/local/bin/control-plane
EXPOSE 8080

CMD ["control-plane"]
