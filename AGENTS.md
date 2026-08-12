# AGENTS.md

Project context for AI coding tools. Read this first; it encodes what the
codebase settled on after a lot of iteration. When in doubt, prefer the
existing patterns over inventing new ones.

## What this is

Telebots is a Rust workspace of Telegram bots (degen and imagine so far),
headed for open source. Bots use **long polling** (no webhooks, no public
URL, no TLS in front) and are deployed as Docker containers on a Hetzner VPS
via Docker Compose.

## Layout & layering

```
bots/<name>/          one self-contained bot: src/, Dockerfile, README.md,
                      .env (gitignored) + .env.example (committed)
crates/botkit/        bot framework: app.rs (dispatcher runner),
                      health.rs (metrics server), telemetry.rs (tracing,
                      panic hook)
crates/core/          shared library: blocks/ (Block, Cell, Line, Change)
                      and money/ (Money, Currency) — the document model
                      and value types; nothing renders domain data
crates/coinmarketcap/ CMC client: standalone crate — lib.rs (re-exports),
                      client.rs (HTTP + wire DTOs), types.rs (public data),
                      error.rs (typed Error); no Telebots dependencies
crates/coingecko/     CoinGecko client: same lib/client/types/error shape
crates/cloudflare-ai/ Cloudflare Workers AI client: same shape
crates/storage/       reusable SQLite storage (async kv + record log)
monitor/              admin dashboard: polls each bot's /metrics (JSON),
                      keeps snapshots in SQLite, serves a TanStack Start
                      dashboard (shadcn/ui + TanStack Charts) on
                      127.0.0.1:3000 + internal JSON API (9110)
scripts/up.sh          env provisioning + compose up (run from anywhere)
docker-compose.yml    one service per bot
justfile              developer commands
```

Client crates (coinmarketcap, coingecko, cloudflare-ai) are **standalone**
— no Telebots dependencies, usable by anyone as crates.io crates. Telebots-
side code — core (document model + value types), botkit (runtime), bots —
sits above them. Bots never reach into another bot's files; shared behavior
goes in `crates/`.

## Non-negotiable conventions

- **No free functions** — with one carve-out: stateless presentation in
  `bots/*/src/render.rs` is a module of plain `fn`s that turn data into
  `Block`s (rendering is consumer-side; see below). Everything else is
  encapsulated in structs/enums/traits and their methods. If you're about
  to write a module-level `fn`, put it on a type instead.
- **Consumer-side rendering**: bots build `Block`s from data in
  `bots/<name>/src/render.rs`; data types and `telebots-core` never know
  how they're presented (there is no `RenderBlock` trait). Commands never
  build strings with `push_str` and never call `send_message` — they
  return blocks or explicit outcomes.
- **Command pattern** (in `bots/<name>/src/commands/`): the `Command` enum is
  the spec (BotCommands derive → parsing, `/help`, Telegram menu). Each
  command is an object with typed arguments: a `parse(raw) -> Result<Self>`
  (validation, usage errors surface to the user) and a `reply` producing an
  explicit outcome. `Command::dispatch` is the single place that sends.
  Imagine's `reply` returns an `Outcome` enum (`Text(Block)` | `Generate`
  intent) — no command calls `send_message`; generation runs in a background
  task spawned by `dispatch`.
- **Env**: per-bot `.env` (gitignored, per machine) loaded into the process
  env by dotenvy via `CARGO_MANIFEST_DIR`; each binary's `Config` derives
  `serde::Deserialize` and reads the process env with the `config` crate
  (`config::Environment`, field renames to the `SCREAMING_SNAKE_CASE` names).
  `.env.example` is the committed template. Containers get env from compose
  `env_file`. **Never commit real secrets** — update the example when adding
  a variable.
- **Persistence**: all state lives in SQLite via `crates/storage` — never
  in-memory. Bots open their own database (`Storage::open(path)`); the
  Imagine bot persists cooldowns (kv) and generation history (record log,
  storing a downscaled JPEG copy per image, not the full PNG).
- **Rust**: nightly pinned in `rust-toolchain.toml`, edition 2024, rustfmt
  via `rustfmt.toml`.
- **Error handling**: library crates (`crates/*`) expose typed `thiserror`
  error enums — one `Error` per crate, re-exported at the root and
  `#[non_exhaustive]`; no `anyhow`, no `Result` type alias (write
  `Result<T, Error>`). `anyhow` lives only in binaries (`bots/*`,
  `monitor/`) and botkit's `reply` glue (it transports the command layer's
  errors and renders them with `{:#}`). Constructors return `Result`;
  panics stay only for provable invariants. Tests return `Result`
  (`anyhow::Result` in binaries, the crate's `Error` in libraries) instead
  of `.unwrap()`.
- **Framework boundaries**: `crates/botkit` is a reusable framework and must
  not know bot-specific facts (no bot names, ports, or name→value tables).
  Identity and runtime knobs are injected via
  `AppConfig { service, version, metrics_port }`. Each bot's own
  `config.rs` owns its defaults (degen 9101, imagine 9102) and reads the
  `TELEBOTS_METRICS_PORT` override.

## Commands

```sh
just check      # fmt + clippy (-D warnings) + test + build — run before finishing
just run        # run degen locally (or: just run <bot>)
just test       # cargo test --workspace
just up         # scripts/up.sh — build and (re)start the compose stack
```

CI runs fmt/clippy/test/build on Linux plus a build+test job on Windows.

## Data sources — verify before coding

These external APIs have tier/path quirks that bite. Fetch the current docs
before writing deserializers; do not assume endpoint availability.

- **CoinMarketCap (keyed, free tier)**: works for `quotes/latest`,
  `price-conversion`, `global-metrics/quotes/latest`, `cryptocurrency/info`,
  and the keyless `fear-and-greed`. **`trending/*` and
  `price-performance-stats` are paid** — the free key returns error 1006.
  Docs: `https://pro-api.coinmarketcap.com/llms.txt`.
- **CMC keyless public API**: prefix any supported path with `/public-api`,
  e.g. `https://pro-api.coinmarketcap.com/public-api/v3/fear-and-greed/latest`.
  No key header on keyless calls. The keyless catalog is a fixed subset —
  check it before assuming an endpoint works keyless.
- **CoinGecko**: `search/trending` (no key) — CMC's trending endpoint is
  paid, so `/trending` uses CoinGecko.
- **Cloudflare Workers AI**: image generation via
  `POST /client/v4/accounts/{id}/ai/run/@cf/black-forest-labs/flux-1-schnell`
  (bearer token). Default responses are a JSON envelope
  `{"result":{"image":"<base64>"},"success":true,...}` — check `success`,
  base64-decode `result.image` (may carry a `data:image/<mime>;base64,`
  prefix). Raw image bytes only come back if the request sends
  `Accept: image/*`. Needs `CLOUDFLARE_API_TOKEN` + `CLOUDFLARE_ACCOUNT_ID`.
  Free tier has a daily quota; verify the model path before assuming it works.
- **teloxide 0.17**: `default-features = false` with `macros`, `rustls`,
  `ctrlc_handler`. API docs: `https://docs.rs/teloxide/0.17.0/teloxide/`.

## Gotchas

- teloxide's `filter_command` **silently drops** updates when a typed
  payload's `FromStr` fails — so the enum keeps raw `String` payloads and
  commands validate in `parse()` (so usage errors reach the user). There is
  no `RequestResult` type in teloxide 0.17; use `ResponseResult`.
- rustls resolves to `aws-lc-rs`, whose C build on Windows needs
  CMake/Go/Perl/NASM — the CI Windows job guards this.
- `scripts/up.sh` resolves the repo root from its own location (no
  `../..` paths, no CWD assumptions). First run copies
  `bots/*/.env.example` → `bots/*/.env` and stops for you to fill secrets;
  re-run to bring the stack up.
- Docker build context excludes secrets (`.dockerignore`: `**/.env`,
  `bots/*/.env`); env reaches containers only via compose `env_file`.
- Persistent bot data (SQLite) lives under `data/` — gitignored and
  volume-mounted into containers; never commit `.db` files. `rusqlite` is
  declared in `crates/storage`, not the workspace.
- Local dev must use a **test-bot token** (prod token only on the VPS); two
  long-polling clients on the same token steal updates from each other.
- Keep the READMEs and `.env.example` files in sync when you add or change
  an environment variable.
