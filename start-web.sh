#!/usr/bin/env bash
set -euo pipefail

# Start Command Center Web.
#
# Usage:
#   ./start-web.sh          Build the frontend (if needed) and run task_server
#                           in release mode, serving the built UI at
#                           http://127.0.0.1:4287
#   ./start-web.sh --dev    Run task_server + the Vite dev server together,
#                           with hot reload, at http://localhost:5173
#   ./start-web.sh --rebuild
#                           Force a fresh frontend build even if web/dist
#                           already exists, then serve as above.

# --- Colors & formatting ---
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

info()    { printf "${BLUE}▸${NC} %s\n" "$1"; }
success() { printf "${GREEN}✓${NC} %s\n" "$1"; }
warn()    { printf "${YELLOW}!${NC} %s\n" "$1"; }
header()  { printf "\n${BOLD}%s${NC}\n" "$1"; }

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

MODE="serve"
case "${1:-}" in
    --dev) MODE="dev" ;;
    --rebuild) MODE="rebuild" ;;
    "") ;;
    *)
        echo "Unknown option: $1" >&2
        echo "Usage: $0 [--dev|--rebuild]" >&2
        exit 1
        ;;
esac

if ! command -v cargo &>/dev/null; then
    warn "cargo not found. Install Rust: https://rustup.rs"
    exit 1
fi

if [[ "$MODE" == "dev" ]]; then
    header "Command Center Web — dev mode (hot reload)"

    info "Starting task_server on http://127.0.0.1:4287 ..."
    cargo run --bin task_server &
    SERVER_PID=$!
    trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

    if [[ ! -d web/node_modules ]]; then
        info "Installing frontend dependencies..."
        (cd web && npm install)
    fi

    success "task_server running (PID $SERVER_PID)"
    info "Starting Vite dev server — open the URL it prints below (usually http://localhost:5173)"
    (cd web && npm run dev)
    exit 0
fi

if [[ "$MODE" == "rebuild" || ! -d web/dist ]]; then
    header "Building frontend"
    if [[ ! -d web/node_modules ]]; then
        info "Installing frontend dependencies..."
        (cd web && npm install)
    fi
    (cd web && npm run build)
    success "Built web/dist"
else
    info "web/dist already exists — skipping build (use --rebuild to force)"
fi

header "Command Center Web"
info "Starting task_server (release) on http://127.0.0.1:4287 ..."
exec cargo run --release --bin task_server
