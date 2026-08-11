# Telebots

A bunch of Telegram bots to make Telegram better.

| Crate                | Description                                   |
| -------------------- | --------------------------------------------- |
| [`degen`](crates/degen) | CoinMarketCap-powered price bot for groups |

## Getting started

The bot uses long polling: it pulls updates from Telegram outbound, so no
public URL, tunnel or reverse proxy is needed.

Env vars live in the committed, encrypted `deploy/env/<bot>.env` files
([dotenvx](https://dotenvx.com/) encryption). They decrypt automatically at
runtime when `.env.keys` sits beside them (gitignored — copy it to any new
machine out-of-band):

```sh
dotenvx run -f deploy/env/degen.env -fk deploy/env/.env.keys -- cargo run -p degen
```

**Use a separate test bot for local dev.** Telegram delivers each update to
only one long-polling client per token, so running the production token
locally while production is also running makes the two steal updates from
each other. Get a throwaway token from [@BotFather](https://t.me/BotFather)
and set it with `dotenvx set TELEBOTS_API_KEY_DEGEN <token> -f deploy/env/degen.env`.

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
(re-encrypts in place), then commit and `git pull && sudo ./deploy/deploy.sh`
on the VPS.

Useful commands:

```sh
docker compose ps              # running bots
docker compose logs -f degen   # follow one bot's logs
docker compose up -d --build   # rebuild + restart after `git pull`
```

Adding a bot (each gets its own container):

1. Add the crate: `crates/<name>/`
2. Copy `crates/degen/Dockerfile` to `crates/<name>/Dockerfile` (change the
   `-p` flag to your crate)
3. `dotenvx encrypt -f deploy/env/<name>.env` (create the encrypted env file)
4. Add a `<name>` service block to [`docker-compose.yml`](./docker-compose.yml)

Notes:

- Only outbound connectivity to `api.telegram.org` and the CoinMarketCap API
  is required; nothing needs to be reachable from the internet, so the VPS
  firewall can drop inbound traffic.
- Local dev: see [Getting started](#getting-started) above — same code,
  no extra setup.

## License

[MIT](./LICENSE)
