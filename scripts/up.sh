#!/usr/bin/env bash
# Builds and (re)starts all bot containers.
# Per-bot env files come from bots/*/.env.example; the first run copies the
# examples into place and stops so you can fill in real values.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "${SCRIPT_DIR}")"
cd "${REPO_DIR}"

created=0
for example in bots/*/.env.example monitor/.env.example; do
    [[ -f "${example}" ]] || continue
    target="${example%.example}"
    if [[ ! -f "${target}" ]]; then
        cp "${example}" "${target}"
        echo "Created ${target} — fill in the secrets."
        created=$((created + 1))
    fi
done

if [[ ${created} -gt 0 ]]; then
    echo
    echo "==> Edit the env files above, then re-run: sudo ./scripts/up.sh"
    exit 0
fi

docker compose up -d --build
docker compose ps
