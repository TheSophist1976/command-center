#!/usr/bin/env bash
set -uo pipefail

# Runs "work my tasks" on a repeating interval, from an interactive shell.
#
# Why this exists instead of a launchd job: Claude Code's MCP connectors
# (Jira, Slack, etc.) are authenticated via OAuth tokens stored in macOS
# Keychain, scoped to the interactive login session. A launchd LaunchAgent
# runs in a different process lineage and can't reach those tokens, so
# MCP-dependent subagents fail silently. A loop running as a child of your
# own Terminal session shares your login session's security context, so it
# has the same Keychain/MCP access you do when running `claude` by hand.
#
# This means the loop must keep running in a real terminal (or a
# detached-but-still-logged-in session like tmux/screen, or `nohup ... &`
# from Terminal) — it stops working the moment that session ends, same as
# any other interactive-shell child process.
#
# Usage:
#   ./work-tasks-loop.sh              Run every 30 minutes (default)
#   ./work-tasks-loop.sh 900          Run every 900 seconds instead
#
#   Foreground (visible in this terminal):
#     ./work-tasks-loop.sh
#
#   Backgrounded but tied to this login session (survives closing the tab,
#   not a reboot or logout):
#     nohup ./work-tasks-loop.sh > /dev/null 2>&1 &
#     disown
#
#   Stop it: find the PID (`pgrep -f work-tasks-loop.sh`) and `kill` it, or
#   Ctrl-C if running in the foreground.

INTERVAL_SECONDS="${1:-1800}"
LOG_FILE="$HOME/Library/Logs/command-center-work-tasks.log"

GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { printf "${BLUE}▸${NC} %s\n" "$1"; }
success() { printf "${GREEN}✓${NC} %s\n" "$1"; }
warn()    { printf "${YELLOW}!${NC} %s\n" "$1"; }

mkdir -p "$(dirname "$LOG_FILE")"

info "Looping \"work my tasks\" every ${INTERVAL_SECONDS}s. Logging to $LOG_FILE. Ctrl-C to stop."

while true; do
    timestamp="$(date '+%Y-%m-%d %H:%M:%S')"
    {
        echo ""
        echo "===== $timestamp ====="
    } >> "$LOG_FILE"

    if claude -p "work my tasks" --allow-dangerously-skip-permissions >> "$LOG_FILE" 2>&1; then
        success "[$timestamp] run finished — see $LOG_FILE"
    else
        warn "[$timestamp] run exited with an error — see $LOG_FILE"
    fi

    sleep "$INTERVAL_SECONDS"
done
