# Telebots — common developer commands
#
#   just --list    show all recipes
#   just check     everything CI runs — run before pushing
#   just run       start the degen bot locally with your dev token
#
# Recipes with a `bot` parameter default to `degen`.

# Format check (CI parity)
fmt:
    cargo fmt --all --check

# Clippy with warnings denied (CI parity)
lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Run the test suite
test:
    cargo test --workspace

# Build the whole workspace
build:
    cargo build --workspace

# Format + lint + test + build — what CI runs
check: fmt lint test build

# Run a bot locally with your personal dev token.
# `deploy/env/<bot>.local.env` shadows the prod token in the committed <bot>.env — dotenvx merges files, first one wins.
run bot='degen':
    dotenvx run -f deploy/env/{{ bot }}.local.env -f deploy/env/{{ bot }}.env -fk deploy/env/.env.keys -- cargo run -p {{ bot }}

# Run with the production token — only while prod isn't polling!
run-prod bot='degen':
    dotenvx run -f deploy/env/{{ bot }}.env -fk deploy/env/.env.keys -- cargo run -p {{ bot }}

# Change a bot's prod env value (re-encrypts the committed file in place)
set key value bot='degen':
    dotenvx set {{ key }} {{ value }} -f deploy/env/{{ bot }}.env

# Set your personal dev token (plaintext, gitignored; creates <bot>.local.env)
set-dev key value bot='degen':
    touch deploy/env/{{ bot }}.local.env
    dotenvx set {{ key }} {{ value }} -f deploy/env/{{ bot }}.local.env -p

# Deploy to the VPS (builds and restarts containers)
deploy:
    ./deploy/deploy.sh

# Follow a bot's container logs
logs bot='degen':
    docker compose logs -f {{ bot }}

# Bare `just` runs the full check
default: check
