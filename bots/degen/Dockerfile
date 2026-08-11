# TLS is rustls (pure Rust), so no OpenSSL/pkg-config system deps are needed.
FROM rust:1-bookworm AS builder

# dotenvx: standalone binary used at runtime to load .env (supports encryption).
RUN curl -sfS https://dotenvx.sh/install.sh | sh

WORKDIR /app

COPY . .

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release -p degen \
    && mkdir -p /out \
    && cp /app/target/release/degen /out/degen

FROM debian:bookworm-slim

# ca-certificates: rustls reads system roots for TLS validation.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/local/bin/dotenvx /usr/local/bin/dotenvx

COPY --from=builder /out/degen /usr/local/bin/degen

# dotenvx decrypts /app/degen.env (mounted by compose) using /app/.env.keys.
CMD ["dotenvx", "run", "-f", "degen.env", "-fk", ".env.keys", "--", "degen"]
