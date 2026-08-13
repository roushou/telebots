# degen

A Telegram crypto bot.

## Setup

```sh
cp bots/degen/.env.example bots/degen/.env   # fill in the keys
mise run bot degen                           # cargo run -p degen; loads its .env
```

Use a separate test bot token from @BotFather for local dev.

| Variable                | Where to get it                                    |
| ----------------------- | -------------------------------------------------- |
| `TELEBOTS_TELEGRAM_API_KEY` | Telegram bot token from [@BotFather]           |
| `COINMARKETCAP_API_KEY` | Free-tier key from [pro.coinmarketcap.com]         |
| `TELEBOTS_METRICS_PORT` | Optional: metrics port (default `9101`)           |

[@BotFather]: https://t.me/BotFather
[pro.coinmarketcap.com]: https://pro.coinmarketcap.com

Add the bot to your group chat and it responds to commands (with or without
`@botname` mention).

## Commands

| Command                  | Description                            |
| ------------------------ | -------------------------------------- |
| `/price btc eth`         | Prices, 24h change, market cap, volume |
| `/convert 100 btc usd`   | Convert an amount between assets       |
| `/market`                | Total cap, BTC/ETH dominance           |
| `/compare btc eth`       | Side-by-side comparison table          |
| `/fear_greed`            | Fear & Greed index                     |
| `/trending`              | Top trending coins                     |
| `/info btc`              | Category, website, description         |
| `/help`                  | Show this help                         |
