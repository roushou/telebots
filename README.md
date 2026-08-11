# Telebots

A bunch of Telegram bots to make Telegram better.

| Crate                | Description                                   |
| -------------------- | --------------------------------------------- |
| [`degen`](bots/degen) | cryptocurrency bot                           |

## Getting started

The bot uses long polling: it pulls updates from Telegram outbound, so no
public URL, tunnel or reverse proxy is needed.

Each bot ships its own `.env.example`. Copy it, fill in the keys, and run:

```sh
cp bots/degen/.env.example bots/degen/.env
just run
```

> [!NOTE]
> **Use a separate test bot for local dev.**
>
> Telegram delivers each update to only one long-polling client per token
> so running the production token locally while production is also running
> makes the two steal updates from each other.

## Deployment (VPS)

Every bot runs in its own Docker container to make it very easy to host on a VPS.

One-time VPS provisioning:

```sh
sudo apt install docker.io docker-compose-v2
sudo systemctl enable --now docker
sudo usermod -aG docker $USER   # optional; re-login to take effect
```

Deploy (first run copies the env examples into place, then builds and starts):

```sh
git clone https://github.com/roushou/telebots /opt/telebots
cd /opt/telebots
sudo ./deploy.sh
```

The first `deploy.sh` run creates `bots/*/.env` from the committed
`.env.example` files and stops there. Fill in each bot's secrets (bot
token, API keys) and re-run `sudo ./deploy.sh`. Containers receive
their env via compose `env_file`; nothing is baked into the image.

**Adding/changing a variable:** edit `bots/<bot>/.env` on the VPS and
restart with `sudo ./deploy.sh` (and add the key to
`bots/<bot>/.env.example` in the repo).

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
3. Add `bots/<name>/.env.example` with its variables
4. Add a `<name>` service block to [`docker-compose.yml`](./docker-compose.yml)

Notes:

- Only outbound connectivity to `api.telegram.org` and the CoinMarketCap API
  is required; nothing needs to be reachable from the internet, so the VPS
  firewall can drop inbound traffic.
- Local dev: see [Getting started](#getting-started) above — same code,
  no extra setup.

## License

[MIT](./LICENSE)
