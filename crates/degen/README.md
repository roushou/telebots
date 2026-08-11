# degen

A CoinMarketCap-powered Telegram bot: query prices and convert amounts.

## Setup

Env vars live in the committed, encrypted `deploy/env/degen.env`; dotenvx
decrypts it at runtime using the gitignored `deploy/env/.env.keys`:

```sh
dotenvx run -f deploy/env/degen.env -fk deploy/env/.env.keys -- cargo run -p degen
```

Change a value with `dotenvx set KEY value -f deploy/env/degen.env`.

| Variable                | Where to get it                                    |
| ----------------------- | -------------------------------------------------- |
| `TELEBOTS_API_KEY_DEGEN` | Telegram bot token from [@BotFather]              |
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
