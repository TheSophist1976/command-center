#!/usr/bin/env bash
set -euo pipefail

# --- Colors & formatting ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

info()    { printf "${BLUE}▸${NC} %s\n" "$1"; }
success() { printf "${GREEN}✓${NC} %s\n" "$1"; }
warn()    { printf "${YELLOW}!${NC} %s\n" "$1"; }
error()   { printf "${RED}✗${NC} %s\n" "$1" >&2; }
header()  { printf "\n${BOLD}%s${NC}\n" "$1"; }

ask_yn() {
    local prompt="$1" default="${2:-n}"
    local yn
    if [[ "$default" == "y" ]]; then
        printf "${BLUE}▸${NC} %s [Y/n] " "$prompt"
    else
        printf "${BLUE}▸${NC} %s [y/N] " "$prompt"
    fi
    read -r yn
    yn="${yn:-$default}"
    [[ "$yn" =~ ^[Yy] ]]
}

# Track status for summary
STATUS_BINARY=""
STATUS_PATH=""
STATUS_CONFIG=""
STATUS_TODOIST=""
STATUS_CLAUDE=""

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# =========================================================
# 1. Pre-flight checks
# =========================================================
header "1. Pre-flight checks"

if ! command -v cargo &>/dev/null; then
    error "cargo is not installed or not in PATH."
    echo "  Install Rust and Cargo: https://rustup.rs"
    echo "  Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi
success "cargo found: $(cargo --version)"

# =========================================================
# 2. Build
# =========================================================
header "2. Building project (release mode)"

if ! cargo build --release; then
    error "Build failed. See errors above."
    exit 1
fi
success "Build complete: target/release/task"

# =========================================================
# 3. Install binary
# =========================================================
header "3. Installing binary"

INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
mkdir -p "$INSTALL_DIR"

cp target/release/task "$INSTALL_DIR/task"
chmod +x "$INSTALL_DIR/task"
success "Installed task to $INSTALL_DIR/task"
STATUS_BINARY="installed → $INSTALL_DIR/task"

# =========================================================
# 4. PATH detection & shell profile
# =========================================================
header "4. PATH configuration"

EXPORT_LINE="export PATH=\"\$HOME/.local/bin:\$PATH\""

if echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    success "$INSTALL_DIR is already in PATH"
    STATUS_PATH="already in PATH"
else
    warn "$INSTALL_DIR is not in your PATH"

    # Detect shell profile
    SHELL_NAME="$(basename "$SHELL")"
    if [[ "$SHELL_NAME" == "zsh" ]]; then
        PROFILE="$HOME/.zshrc"
    elif [[ -f "$HOME/.bash_profile" ]]; then
        PROFILE="$HOME/.bash_profile"
    else
        PROFILE="$HOME/.bashrc"
    fi

    # Check if already in profile (idempotency)
    if [[ -f "$PROFILE" ]] && grep -qF '.local/bin' "$PROFILE"; then
        success "PATH export already present in $PROFILE"
        STATUS_PATH="already in $PROFILE"
    elif ask_yn "Add to $PROFILE?"; then
        echo "" >> "$PROFILE"
        echo "# Added by task-manager deploy.sh" >> "$PROFILE"
        echo "$EXPORT_LINE" >> "$PROFILE"
        success "Added to $PROFILE"
        warn "Run 'source $PROFILE' or open a new terminal to apply"
        STATUS_PATH="added to $PROFILE"
    else
        echo ""
        info "Add this line to your shell profile manually:"
        echo "  $EXPORT_LINE"
        STATUS_PATH="skipped (manual setup needed)"
    fi
fi

# =========================================================
# 5. Config: default-dir
# =========================================================
header "5. Configuration"

CONFIG_DIR="$HOME/.config/task-manager"
CONFIG_FILE="$CONFIG_DIR/config.md"

current_dir=""
if [[ -f "$CONFIG_FILE" ]]; then
    current_dir="$(grep '^default-dir:' "$CONFIG_FILE" 2>/dev/null | sed 's/^default-dir:[[:space:]]*//' || true)"
fi

if [[ -n "$current_dir" ]]; then
    success "default-dir: $current_dir"
    STATUS_CONFIG="configured → $current_dir"
else
    info "No default task directory configured."
    printf "${BLUE}▸${NC} Enter the directory for your tasks (e.g., ~/tasks): "
    read -r task_dir

    if [[ -z "$task_dir" ]]; then
        warn "Skipped default-dir configuration"
        STATUS_CONFIG="skipped"
    else
        # Expand tilde
        task_dir="${task_dir/#\~/$HOME}"

        if [[ ! -d "$task_dir" ]]; then
            if ask_yn "Directory '$task_dir' does not exist. Create it?"; then
                mkdir -p "$task_dir"
                success "Created $task_dir"
            fi
        fi

        mkdir -p "$CONFIG_DIR"
        if [[ -f "$CONFIG_FILE" ]]; then
            echo "default-dir: $task_dir" >> "$CONFIG_FILE"
        else
            printf "# task-manager config\n\ndefault-dir: %s\n" "$task_dir" > "$CONFIG_FILE"
        fi
        success "default-dir set to $task_dir"
        STATUS_CONFIG="configured → $task_dir"
    fi
fi

# =========================================================
# 6. Auth: Todoist token
# =========================================================
header "6. Integrations"

TODOIST_TOKEN_FILE="$CONFIG_DIR/todoist_token"

if [[ -f "$TODOIST_TOKEN_FILE" ]] && [[ -s "$TODOIST_TOKEN_FILE" ]]; then
    success "Todoist: configured"
    STATUS_TODOIST="configured"
else
    if ask_yn "Set up Todoist sync?"; then
        echo ""
        info "Find your API token at:"
        echo "  https://app.todoist.com/app/settings/integrations/developer"
        echo ""
        printf "${BLUE}▸${NC} Paste your Todoist API token: "
        read -r todoist_token

        if [[ -n "$todoist_token" ]]; then
            mkdir -p "$CONFIG_DIR"
            printf "%s" "$todoist_token" > "$TODOIST_TOKEN_FILE"
            chmod 600 "$TODOIST_TOKEN_FILE"
            success "Todoist token saved"
            STATUS_TODOIST="configured"
        else
            warn "Empty token, skipped"
            STATUS_TODOIST="skipped"
        fi
    else
        info "Skipped Todoist setup"
        STATUS_TODOIST="skipped"
    fi
fi

# --- Claude API key ---

CLAUDE_KEY_FILE="$CONFIG_DIR/claude_api_key"

if [[ -n "${ANTHROPIC_API_KEY:-}" ]]; then
    success "Claude API: configured (env var)"
    STATUS_CLAUDE="configured (env var)"
elif [[ -f "$CLAUDE_KEY_FILE" ]] && [[ -s "$CLAUDE_KEY_FILE" ]]; then
    success "Claude API: configured (file)"
    STATUS_CLAUDE="configured (file)"
else
    if ask_yn "Set up Claude API key for NLP features?"; then
        echo ""
        info "Get your API key at:"
        echo "  https://console.anthropic.com/settings/keys"
        echo ""
        printf "${BLUE}▸${NC} Paste your Claude API key: "
        read -r claude_key

        if [[ -n "$claude_key" ]]; then
            mkdir -p "$CONFIG_DIR"
            printf "%s" "$claude_key" > "$CLAUDE_KEY_FILE"
            chmod 600 "$CLAUDE_KEY_FILE"
            success "Claude API key saved"
            STATUS_CLAUDE="configured (file)"
        else
            warn "Empty key, skipped"
            STATUS_CLAUDE="skipped"
        fi
    else
        info "Skipped Claude API setup"
        STATUS_CLAUDE="skipped"
    fi
fi

# =========================================================
# 7. Summary
# =========================================================
header "Setup Complete"
echo ""
printf "  %-16s %s\n" "Binary:" "$STATUS_BINARY"
printf "  %-16s %s\n" "PATH:" "$STATUS_PATH"
printf "  %-16s %s\n" "Config:" "$STATUS_CONFIG"
printf "  %-16s %s\n" "Todoist:" "$STATUS_TODOIST"
printf "  %-16s %s\n" "Claude API:" "$STATUS_CLAUDE"
echo ""
success "Done! Run 'task' to get started."
