# Telebots

A bunch of Telegram bots to make Telegram better.

| Crate                | Description                                   |
| -------------------- | --------------------------------------------- |
| [`degen`](bots/degen) | cryptocurrency bot                           |

## Getting started

The bot uses long polling: it pulls updates from Telegram outbound, so no
public URL, tunnel or reverse proxy is needed.

Env vars live in the committed, encrypted `deploy/env/<bot>.env` files
([dotenvx](https://dotenvx.com/) encryption) — one file per bot, holding
that bot's production token and API keys. They decrypt automatically at
runtime when `.env.keys` sits beside them.

For local development, create `deploy/env/<bot>.local.env` with your own
bot token `TELEBOTS_TELEGRAM_API_KEY=<your-token>`.

Then run the bot.

```sh
just run
```

> [!NOTE]
> **Use a separate test bot for local dev.**
>
> Telegram delivers each update to only one long-polling client per token
> so running the production token locally while production is also running
> makes the two steal updates from each other.
>
> Every developer keeps their own throwaway token from [@BotFather](https://t.me/BotFather)
> in a gitignored local override file and dotenvx merges the override on top of the committed file — first file wins:

```sh
just set-dev TELEBOTS_TELEGRAM_API_KEY <token>   # writes deploy/env/degen.local.env
just run                                          # your dev token; prod untouched
```

For another bot, pass it as the last argument (`just set-dev ... crypto`,
`just run crypto`). Without a local override, `just run` falls back to the
prod token — so don't run a bot locally while its VPS container is up.

## Deployment (VPS)

Every bot runs in its own Docker container on the Hetzner VPS, orchestrated
by [Docker Compose](./docker-compose.yml). Long polling means containers only
need outbound HTTPS — no public URL, no TLS, no reverse proxy, no exposed
ports.

One-time VPS provisioning:

```sh
sudo apt install docker.io docker-compose-v2
sudo systemctl enable --now docker
sudo usermod -aG docker $USER   # optional; re-login to take effect
```

Deploy (first run checks keys, then builds and starts):

```sh
git clone https://github.com/roushou/telebots /opt/telebots
cd /opt/telebots
# one-time: copy the gitignored decryption keys from your dev machine
scp deploy/env/.env.keys root@<vps>:/opt/telebots/deploy/env/.env.keys
sudo ./deploy/deploy.sh
```

Env is fully provided by the committed encrypted `deploy/env/*.env` files;
containers mount them and dotenvx decrypts at runtime. The `.env.keys` files
are gitignored and never enter the image.

**Adding/changing a variable:** `dotenvx set KEY value -f deploy/env/<bot>.env`
(or `just set KEY value <bot>`) (re-encrypts in place), then commit and
`git pull && sudo ./deploy/deploy.sh` on the VPS.

Useful commands:

```sh
docker compose ps              # running bots
docker compose logs -f degen   # follow one bot's logs
docker compose up -d --build   # rebuild + restart after `git pull`
```

Adding a bot (each gets its own container):

1. Add the bot: `bots/<name>/`
2. Copy `bots/degen/Dockerfile` to `bots/<name>/Dockerfile` (change the
   `-p` flag to your crate)
3. Create the encrypted env file (generates a new keypair):
   `touch deploy/env/<name>.env && dotenvx set TELEBOTS_TELEGRAM_API_KEY <token> -f deploy/env/<name>.env`
4. Add a `<name>` service block to [`docker-compose.yml`](./docker-compose.yml)
5. Distribute the new key out-of-band: `scp deploy/env/.env.keys root@<vps>:/opt/telebots/deploy/env/.env.keys`
   — teammates just run `just set-dev TELEBOTS_TELEGRAM_API_KEY <token> <name>`
   for their own dev token; no new key needed on their side

Notes:

- Only outbound connectivity to `api.telegram.org` and the CoinMarketCap API
  is required; nothing needs to be reachable from the internet, so the VPS
  firewall can drop inbound traffic.
- Local dev: see [Getting started](#getting-started) above — same code,
  no extra setup.

## License

[MIT](./LICENSE)
