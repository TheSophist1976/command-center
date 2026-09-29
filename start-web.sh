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
#   ./start-web.sh --profile home
#                           Serve a specific profile (combines with the above;
#                           set TASK_SERVER_PORT to run several side by side).

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
SERVER_ARGS=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --dev) MODE="dev" ;;
        --rebuild) MODE="rebuild" ;;
        --profile)
            [[ -n "${2:-}" ]] || { echo "--profile needs a name" >&2; exit 1; }
            SERVER_ARGS+=(--profile "$2"); shift ;;
        *)
            echo "Unknown option: $1" >&2
            echo "Usage: $0 [--dev|--rebuild] [--profile NAME]" >&2
            exit 1
            ;;
    esac
    shift
done
PORT="${TASK_SERVER_PORT:-4287}"

if ! command -v cargo &>/dev/null; then
    warn "cargo not found. Install Rust: https://rustup.rs"
    exit 1
fi

if [[ "$MODE" == "dev" ]]; then
    header "Command Center Web — dev mode (hot reload)"

    info "Starting task_server on http://127.0.0.1:$PORT ..."
    cargo run --bin task_server -- "${SERVER_ARGS[@]}" &
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
info "Starting task_server (release) on http://127.0.0.1:$PORT ..."
exec cargo run --release --bin task_server -- "${SERVER_ARGS[@]}"
