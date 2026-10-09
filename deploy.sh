#!/usr/bin/env bash
# DEPRECATED: releases are built and published by GitHub Actions on a `v*` tag.
# See "Releasing" in README.md. To install or update, use install.sh / `task update`.
echo "deploy.sh is deprecated. Releases are cut by pushing a v* tag; see 'Releasing' in README.md." >&2
echo "Install:  curl -fsSL https://raw.githubusercontent.com/TheSophist1976/command-center-releases/main/install.sh | sh" >&2
echo "Update:   task update" >&2
echo "From source: cd web && npm ci && npm run build && cd .. && cargo build --release" >&2
exit 1
