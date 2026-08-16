# Imagine

A Telegram bot that generates images from text prompts (via Cloudflare
Workers AI). Sends a "🎨 generating…" placeholder, then delivers the photo;
recent generations are stored in SQLite and listed with `/history`.

Users can pick a text-to-image model per request by prefixing the prompt
(`/imagine <model> <prompt>`); `/imagine <prompt>` uses the default
(`flux-1-schnell`).

## Setup

```sh
cp bots/imagine/.env.example bots/imagine/.env   # fill in the keys
mise run bot imagine                             # cargo run -p imagine
```

Use a separate test bot token from @BotFather for local dev.

| Variable                | Where to get it                                             |
| ----------------------- | ----------------------------------------------------------- |
| `TELEBOTS_TELEGRAM_API_KEY` | Telegram bot token from [@BotFather]                    |
| `CLOUDFLARE_API_TOKEN`  | API token with Workers AI access, dash.cloudflare.com       |
| `CLOUDFLARE_ACCOUNT_ID` | Account id from the Workers AI dashboard URL                |
| `IMAGINE_DB_PATH`       | SQLite path (default `imagine.db` locally; `/data/imagine.db` in the container, set in docker-compose.yml) |
| `TELEBOTS_METRICS_PORT` | Optional: metrics port (default `9102`)                                |

[@BotFather]: https://t.me/BotFather

## Commands

| Command                     | Description                          |
| --------------------------- | ------------------------------------ |
| `/imagine [model] <prompt>` | Generate an image from a prompt      |
| `/history`                  | List your recent generations         |
| `/stats`                    | Bot uptime, commands, jobs, panics   |
| `/help`                     | Show this help                       |

## Models

Pass one of these tokens before the prompt, e.g.
`/imagine flux-2-dev a cat in a spacesuit`. Without a token,
`flux-1-schnell` is used.

| Token(s)                              | Model                       |
| ------------------------------------- | --------------------------- |
| `flux-2-dev`, `flux2dev`              | Flux 2 Dev                  |
| `flux-1-schnell`, `schnell`           | Flux 1 Schnell (default)    |
| `flux-2-klein-4b`, `klein-4b`         | Flux 2 Klein 4B             |
| `flux-2-klein-9b`, `klein-9b`         | Flux 2 Klein 9B             |
| `sd-xl-lightning`, `sdxl-lightning`   | SDXL Lightning              |
| `dreamshaper-8-lcm`, `dreamshaper`    | Dreamshaper 8 LCM           |
| `sd-xl-base`, `sdxl-base`             | SDXL Base 1.0               |

## Notes

- Generation runs in a background task; the bot replies instantly with a
  placeholder and delivers the photo (as a reply) when ready.
- One image per user per 30s; cooldowns and history persist in SQLite (no
  in-memory state), so they survive restarts.
- The image provider is behind a `Generator` enum (`src/generator.rs`) —
  swapping providers touches only that module and the `Ctx` wiring. Model
  names live in `cloudflare-ai::ImageModel` (`crates/cloudflare-ai`); the bot
  never hard-codes `@cf/...` paths.
- Cloudflare's free Workers AI tier has a daily quota; if exceeded, requests
  fail with a Cloudflare error surfaced as `⚠️`.
