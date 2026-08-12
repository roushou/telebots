# TLS is rustls (pure Rust), so no OpenSSL/pkg-config system deps are needed.
FROM rust:1-bookworm AS rust-builder

WORKDIR /app

COPY . .

RUN --mount=type=cache,sharing=locked,target=/usr/local/cargo/registry \
    --mount=type=cache,sharing=locked,target=/app/target \
    cargo build --release -p monitor \
    && mkdir -p /out \
    && cp /app/target/release/monitor /out/monitor

FROM oven/bun:1 AS web-builder

WORKDIR /app/monitor/web

COPY monitor/web/package.json monitor/web/bun.lock ./
RUN bun install --frozen-lockfile

COPY monitor/web/ .
RUN bun run build

FROM node:22-bookworm-slim

# ca-certificates: rustls reads system roots for TLS validation.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=rust-builder /out/monitor /usr/local/bin/monitor
COPY --from=web-builder /app/monitor/web/.output /app/web
COPY monitor/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

EXPOSE 3000

CMD ["docker-entrypoint.sh"]
