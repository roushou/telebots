# Bud

A Telegram bot you can talk to — like ChatGPT, but inside Telegram. Bud
answers free-form messages with a text-generation model from Cloudflare
Workers AI, remembers the conversation per chat, and persists everything in
SQLite so it survives restarts.

In a private chat, just say hello. In groups, bud only answers when you
`@bud`-mention it or reply to one of its messages.

## Setup

```sh
cp bots/bud/.env.example bots/bud/.env   # fill in the keys
mise run bot bud                         # cargo run -p bud
```

Use a separate test bot token from @BotFather for local dev.

| Variable                    | Where to get it                                             |
| --------------------------- | ----------------------------------------------------------- |
| `TELEBOTS_TELEGRAM_API_KEY` | Telegram bot token from [@BotFather]                        |
| `CLOUDFLARE_API_TOKEN`      | API token with Workers AI access, dash.cloudflare.com       |
| `CLOUDFLARE_ACCOUNT_ID`     | Account id from the Workers AI dashboard URL                |
| `BUD_DB_PATH`               | SQLite path (default `bud.db` locally; `/data/bud.db` in the container) |
| `BUD_SYSTEM_PROMPT`         | Optional: default personality (per-chat override with `/system`) |
| `BUD_MAX_HISTORY`           | Optional: prior messages kept in context (default 20)       |
| `TELEBOTS_METRICS_PORT`     | Optional: metrics port (default `9103`)                     |

[@BotFather]: https://t.me/BotFather

## Commands

| Command            | Description                          |
| ------------------ | ------------------------------------ |
| *(free text)*      | Talk to bud                          |
| `/reset`           | Clear the conversation               |
| `/history`         | Show your recent conversation        |
| `/model <name>`    | Pick the AI model                    |
| `/system <prompt>` | Set bud's personality (`/system reset` restores the default) |
| `/stats`           | Bot uptime, commands, jobs, panics   |
| `/help`            | Show this help                       |

## Models

Pass one of these tokens to `/model`, e.g. `/model deepseek-r1`. Without a
choice, `llama-3.1-8b` is used.

| Token(s)            | Model                       |
| ------------------- | --------------------------- |
| `llama-3.1-8b`, `8b` | Llama 3.1 8B (default)     |
| `llama-3.2-3b`, `3b` | Llama 3.2 3B — fastest      |
| `llama-3.3-70b`, `70b` | Llama 3.3 70B — best quality |
| `deepseek-r1`, `r1` | DeepSeek R1 32B — reasoning |

## Notes

- Generation runs in a background task; bud replies instantly with a
  "✍️ thinking…" placeholder and edits it with the answer when ready.
- One message per user per 10s; cooldowns, history, and settings persist in
  SQLite (no in-memory state), so they survive restarts.
- The text provider is behind a `Generator` enum (`src/generator.rs`) —
  swapping providers touches only that module and the `Ctx` wiring. Model
  names live in `cloudflare-ai::TextModel`; the bot never hard-codes
  `@cf/...` paths.
- Cloudflare's free Workers AI tier has a daily quota; if exceeded, requests
  fail with a Cloudflare error surfaced as `⚠️`.
