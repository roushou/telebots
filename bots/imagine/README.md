# Imagine

A Telegram bot that generates images from text prompts (via Cloudflare
Workers AI, Flux). Sends a "🎨 generating…" placeholder, then delivers the
photo; recent generations are stored in SQLite and listed with `/history`.

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
| `/imagine <prompt>`         | Generate an image from a prompt      |
| `/history`                  | List your recent generations         |
| `/help`                     | Show this help                       |

## Notes

- Generation runs in a background task; the bot replies instantly with a
  placeholder and delivers the photo (as a reply) when ready.
- One image per user per 30s; cooldowns and history persist in SQLite (no
  in-memory state), so they survive restarts.
- The image provider is behind a `Generator` enum (`src/generator.rs`) —
  swapping providers touches only that module and the `Ctx` wiring.
- Cloudflare's free Workers AI tier has a daily quota; if exceeded, requests
  fail with a Cloudflare error surfaced as `⚠️`.
