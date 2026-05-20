FROM rust:1.86-slim-bookworm AS builder

WORKDIR /app

COPY Cargo.toml ./
COPY src/ ./src/

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/kagi-proxy /kagi-proxy

USER 65534

EXPOSE 3000

ENTRYPOINT ["/kagi-proxy"]