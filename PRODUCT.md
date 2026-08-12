# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Existing: TanStack Start (React 19-less, React 18) + TanStack Router + shadcn/ui + Tailwind CSS, built with Vite/Nitro and served by Node inside the monitor container.

## Users

The Telebots maintainers (roushou and collaborators) — developers checking that their own Telegram bots are alive. They reach the dashboard over Tailscale, on a laptop, usually when something feels off or after a deploy.

## Product Purpose

A single place to see whether every bot is healthy, and — when one isn't — to see exactly when it went down and why. The monitor polls each bot's `/metrics` JSON every 30 seconds, stores snapshots in SQLite, and renders them.

## Positioning

It is wired to the bots' own `/metrics` endpoints and keeps a rolling snapshot history, so "what happened" is answerable from stored data rather than from Telegram logs or guesswork.

## Operating Context

Internal, low-traffic, low-ceremony. Two processes in one Docker container: the Rust JSON API (in-container port 9110) and the TanStack Start dashboard (host port 3000). Monitoring is passive: the dashboard is checked on demand; there is no push alerting.

## Capabilities and Constraints

- Overview of all bots and a per-bot detail with history (endpoint exists: `GET /api/bots/<name>/history?limit=100`, capped at 1000 snapshots).
- Snapshot fields: version, uptime_secs, telegram reachability, last heartbeat/update age, jobs active, jobs failed total, panics total, and an error string when unreachable.
- ~2–10 bots expected. Polling cadence is 30s.
- Light and dark both supported; dark is the natural default for an ops board.
- Passive only — no push notifications out of scope unless the user asks.

## Brand Commitments

Name: Telebots. Visual reference pinned by the user: shadcn/ui blocks (dashboard patterns) played straight, as the category standard.

## Evidence on Hand

Real per-bot metrics and error strings, and per-bot snapshot history. No logo or imagery exists; none should be invented.

## Product Principles

1. Health at a glance — a down bot is unmistakable within seconds.
2. Every red state traces to evidence — an error message and a point on a timeline.
3. Consistency over novelty — standard dashboard grammar, tuned for scanning.
4. Show only what the bots report — never fabricate or infer status that isn't in the data.
5. Low ceremony — fast to load, fast to read, nothing to configure.

## Accessibility & Inclusion

Status must be readable without color alone (badge + icon + text); full keyboard navigation; visible focus states; dark and light contrast within bounds.
