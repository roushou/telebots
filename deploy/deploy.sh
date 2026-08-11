#!/usr/bin/env bash
# Builds and (re)starts all bot containers.
# Env comes from the committed encrypted deploy/env/*.env files; the matching
# .env.keys files are gitignored, so copy them to the VPS out-of-band first.
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${REPO_DIR}"

if [[ ! -f "deploy/env/.env.keys" ]]; then
    echo "missing deploy/env/.env.keys (gitignored, not in the repo) — copy it:"
    echo "  scp deploy/env/.env.keys root@<vps>:/opt/telebots/deploy/env/.env.keys"
    echo "==> Then re-run: sudo ./deploy/deploy.sh"
    exit 1
fi

docker compose up -d --build
docker compose ps
