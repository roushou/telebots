# TLS is rustls (pure Rust), so no OpenSSL/pkg-config system deps are needed.
FROM rust:1-bookworm AS builder

WORKDIR /app

COPY . .

RUN --mount=type=cache,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=locked,target=/app/target \
    cargo build --release -p imagine \
    && mkdir -p /out \
    && cp /app/target/release/imagine /out/imagine

FROM debian:bookworm-slim

# ca-certificates: rustls reads system roots for TLS validation.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /out/imagine /usr/local/bin/imagine

CMD ["imagine"]
