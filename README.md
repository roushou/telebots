# Telebots

A bunch of Telegram bots to make Telegram better.

| Crate                | Description                                   |
| -------------------- | --------------------------------------------- |
| [`degen`](bots/degen)   | cryptocurrency bot                           |
| [`imagine`](bots/imagine) | AI image generation from prompts           |
| [`monitor`](monitor) | admin dashboard: per-bot status + history     |

## Getting started

Bots use long polling: they pull updates from Telegram outbound, so no
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

Bring the stack up (first run copies the env examples into place, then builds and starts):

```sh
git clone https://github.com/roushou/telebots /opt/telebots
cd /opt/telebots
sudo ./scripts/up.sh
```

The first `up.sh` run creates `bots/*/.env` from the committed
`.env.example` files and stops there. Fill in each bot's secrets (bot
token, API keys) and re-run `sudo ./scripts/up.sh`. Containers receive
their env via compose `env_file`; nothing is baked into the image.

**Adding/changing a variable:** edit `bots/<bot>/.env` on the VPS and
restart with `sudo ./scripts/up.sh` (and add the key to
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

- Only outbound connectivity is required (`api.telegram.org`, the
  CoinMarketCap API, `api.cloudflare.com`); nothing needs to be reachable
  from the internet, so the VPS firewall can drop inbound traffic.
- Bots with persistent state (imagine) mount a data volume
  (`./data/<bot>:/data`); the SQLite database lives there, survives
  restarts, and is gitignored.
- Local dev: see [Getting started](#getting-started) above — same code,
  no extra setup.

## Monitoring

The `monitor` service polls every bot's `/metrics` endpoint (JSON),
stores status snapshots in SQLite, and serves an admin dashboard
(TanStack Start, shadcn/ui + TanStack Charts) + JSON API. The dashboard
binds `127.0.0.1:3000`; the JSON API stays internal to the container. See
[monitor/README.md](monitor/README.md).

## License

[MIT](./LICENSE)
