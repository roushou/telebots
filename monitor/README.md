# monitor

The admin dashboard: polls every bot's `/metrics` endpoint (JSON),
stores status snapshots in SQLite, and serves a TanStack Start dashboard
(SSR) plus a JSON API.

Two processes run in the monitor container:

| Process | Port | Reachable from |
| ------- | ---- | -------------- |
| Rust JSON API | `9110` | container only |
| TanStack Start dashboard (Nitro/Node) | `3000` | host (`127.0.0.1:3000`) |

The dashboard's server functions call the JSON API over the container's
loopback interface, so nothing extra is exposed.

## Setup

```sh
mise run bot monitor                   # cargo run -p monitor (the JSON API)
cd monitor/web && bun install                  # one-time: fetch JS deps (bun from mise)
mise run web                                   # dashboard dev server on :3000
```

The dev server fetches bot data from `http://127.0.0.1:9110` (the local
monitor). Override with `MONITOR_API_URL` if the API lives elsewhere.

| Variable          | Where it's set                                              |
| ----------------- | ----------------------------------------------------------- |
| `MONITOR_BOTS`    | Bot `/metrics` endpoints. Local dev: `mise.toml` (`http://localhost:9101/metrics`); container: `docker-compose.yml` (compose network URLs) |
| `MONITOR_DB_PATH` | SQLite path (default `monitor.db` locally; `/data/monitor.db` in the container via `docker-compose.yml`) |
| `MONITOR_PORT`    | JSON API port (default `9110`; set in `docker-compose.yml`) |
| `MONITOR_API_URL` | Optional: base URL the dashboard uses to reach the JSON API (default `http://127.0.0.1:9110`; set in `mise.toml` for the `web` task) |

## API

| Endpoint                          | Description                        |
| --------------------------------- | ---------------------------------- |
| `GET /healthz`                    | Monitor liveness                   |
| `GET /api/bots`                   | Newest status snapshot per bot     |
| `GET /api/bots/<name>/history?limit=100` | Recent snapshots, newest first |

The dashboard is a TanStack Start app under `web/`. `bun run build`
produces a Nitro server build in `web/.output`; the Docker image runs
`node web/.output/server/index.mjs` next to the Rust API (see
`docker-entrypoint.sh`).

### Dashboard

- **Overview** — stat cards (bots / up / down / last snapshot), a 24h
  per-bot health strip, and a bot grid (with a table view toggle) with
  per-bot availability sparklines.
- **Bot detail** (`/bots/<name>`) — availability, jobs, and panics over a
  1h/6h/24h window (from snapshot history), plus restart events and a
  merged error/incident log.
- Dark and light themes (dark default), a `⌘K` command palette, and a
  collapsible sidebar. Charts are TanStack Charts; UI is shadcn/ui.

## Deployment

Each bot exposes `/metrics` on `0.0.0.0:<TELEBOTS_METRICS_PORT>` (degen
9101, imagine 9102, overridable). The monitor container reaches them over
the compose network; the dashboard binds `127.0.0.1:3000` on the host and
the JSON API stays internal to the container.

Dev access via Tailscale (no public ports):

```sh
tailscale up                                   # on the VPS
sudo tailscale serve --bg 3000                 # https://<host>.<tailnet>.ts.net → 127.0.0.1:3000
```
