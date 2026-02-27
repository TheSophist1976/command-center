## Context

The `task` binary is a Rust CLI/TUI app built with Cargo. Configuration lives at `~/.config/task-manager/config.md` (key-value format), and auth tokens are stored as plain files in the same directory (`todoist_token`, `claude_api_key`). The Claude API key can also come from the `ANTHROPIC_API_KEY` env var. There is currently no install or setup automation — users must manually build, copy the binary, and configure credentials independently.

## Goals / Non-Goals

**Goals:**
- Single command (`./deploy.sh`) to build, validate setup, and install
- Interactive guided setup for missing config/auth — no silent failures
- Idempotent — safe to re-run for upgrades or first-time setup
- Works on macOS and common Linux distros (bash 3.2+)

**Non-Goals:**
- Windows support
- Package manager distribution (Homebrew formula, apt/rpm packages)
- Compiling for different target architectures (cross-compilation)
- Modifying any Rust source code

## Decisions

### 1. Pure bash script (no external dependencies beyond cargo)

The script will be a single `deploy.sh` file using only bash builtins, `cp`, `mkdir`, and `chmod`. No Python, no Makefile, no additional tooling.

**Rationale**: Every developer with this repo already has bash and cargo. Adding a build tool dependency would increase friction rather than reduce it.

### 2. Install location: `~/.local/bin` (default) with override

Default to `~/.local/bin` because:
- User-writable without `sudo`
- Follows the [XDG Base Directory](https://specifications.freedesktop.org/basedir-spec/latest/) convention
- Already on PATH for many modern Linux distros

Allow override via `INSTALL_DIR` env var (e.g., `INSTALL_DIR=/usr/local/bin ./deploy.sh`) for users who prefer a system-wide install.

**Alternative considered**: `/usr/local/bin` — requires sudo, which complicates the interactive config/auth prompts and is unnecessarily privileged.

### 3. PATH detection and shell profile update

After installing the binary, the script will:
1. Check if the install directory is already in `$PATH`
2. If not, detect the user's shell profile file (`~/.zshrc`, `~/.bashrc`, or `~/.bash_profile`)
3. Ask the user for consent before appending `export PATH="$HOME/.local/bin:$PATH"`
4. Advise the user to `source` their profile or open a new terminal

**Rationale**: Silently modifying dotfiles is a poor user experience. Asking first respects the user's setup.

### 4. Config and auth checks are non-blocking

- `default-dir` config: Prompted if missing, since the app needs a storage directory to function. The script will validate the directory exists (or offer to create it).
- Todoist token: Optional. Ask "Do you want to set up Todoist sync? (y/N)". Skip on decline.
- Claude API key: Optional. Check `ANTHROPIC_API_KEY` env var first. If not set, ask "Do you want to set up the Claude API key for NLP features? (y/N)". Skip on decline.

**Rationale**: Only `default-dir` is required for basic operation. Blocking on optional integrations would frustrate users who just want the core task manager.

### 5. Script flow order

```
1. Pre-flight checks (cargo installed?)
2. cargo build --release
3. Copy binary to install dir
4. PATH check + shell profile update
5. Config: default-dir
6. Auth: Todoist token (optional)
7. Auth: Claude API key (optional)
8. Summary of what was set up
```

Build and install first, then configure. This way the user has a working binary even if they ctrl-C during the config prompts.

## Risks / Trade-offs

- **Shell profile detection is heuristic** → Mitigated by always asking before modifying, and printing the exact line that would be added so the user can do it manually.
- **`~/.local/bin` may not exist on older systems** → The script creates it if needed. Minimal risk since `mkdir -p` is safe.
- **Re-running overwrites the binary without backup** → Acceptable because the source is always in the repo and can be rebuilt. Not worth the complexity of versioned backups for a dev tool.
- **Auth tokens stored as plaintext files** → This is the existing design (with 0600 permissions). The deploy script follows the same pattern. Out of scope to change the storage mechanism here.
