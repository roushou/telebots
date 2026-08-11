# Telebots — common developer commands
#
#   just --list    show all recipes
#   just check     everything CI runs — run before pushing
#   just run       start the degen bot locally
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

# Run a bot locally with your dev token from .env (dotenvy loads it)
run bot='degen':
    cargo run -p {{ bot }}

# Deploy to the VPS (builds and restarts containers)
deploy:
    ./deploy.sh

# Follow a bot's container logs
logs bot='degen':
    docker compose logs -f {{ bot }}

# Bare `just` runs the full check
default: check
