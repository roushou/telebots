# degen

A Telegram crypto bot.

## Setup

```sh
just run      # local dev
just run-prod # prod token — only while prod isn't polling
```

Set your personal dev token with `just set-dev TELEBOTS_TELEGRAM_API_KEY <token>`.
Change a prod value with `dotenvx set KEY value -f deploy/env/degen.env`.

| Variable                | Where to get it                                    |
| ----------------------- | -------------------------------------------------- |
| `TELEBOTS_TELEGRAM_API_KEY` | Telegram bot token from [@BotFather]           |
| `COINMARKETCAP_API_KEY` | Free-tier key from [pro.coinmarketcap.com]         |

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
