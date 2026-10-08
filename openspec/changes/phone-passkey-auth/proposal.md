## Why

The web client now has a phone layout, but `task serve` only answers on `127.0.0.1` and rejects any Host header that isn't `localhost`/`127.0.0.1`. Reaching it from a phone means putting it behind something like `tailscale serve`, and then anyone who can reach that hostname gets full read/write access to every task, note and agent instruction. The phone needs a way to prove it belongs to the owner.

## What Changes

- **`remote-host` config key** (e.g. `desk.tail1234.ts.net`): the one extra hostname `task serve` will answer for, typically via `tailscale serve`. Unset means today's behaviour (local only).
- **Local vs remote requests**: a request is *local* when its Host is `localhost`/`127.0.0.1` and it carries no proxy forwarding headers; local requests keep working with no sign-in. A request for `remote-host` is *remote* and needs a session; anything else is still rejected with 403.
- **Passkey (WebAuthn) sign-in** for remote requests: ES256 passkeys with user verification (Face ID / Touch ID), discoverable so there's no username. Verified in pure Rust (`p256`, `ciborium`) — no OpenSSL.
- **Pairing**: a phone can only register a passkey with a single-use, 10-minute pairing code created from the desktop (Settings → Devices → Pair a phone), shown as a QR code and a link.
- **Sessions**: a successful sign-in sets a 30-day sliding `HttpOnly; Secure; SameSite=Strict` cookie; only the SHA-256 of the token is stored.
- **Device management** (desktop only): list paired passkeys, revoke one (which ends its sessions).
- **Remote restrictions**: remote requests can't install updates, open notes in the desktop editor, or pair/revoke devices; remote writes must carry a matching `Origin`.
- **Web client**: sign-in screen and pairing screen on the phone, Sign out in the drawer, Devices dialog on the desktop.

## Capabilities

### New Capabilities
- `web-remote-auth`: remote host config, request classification, passkey pairing and sign-in, sessions, device management, remote restrictions

### Modified Capabilities
- `app-config`: new `remote-host` key

## Impact

- `src/server.rs` — replace `validate_host` with the classify/authorize middleware; mount `/api/auth/*`; `AppState` gains the auth store and remote host
- `src/web_auth.rs` (new) — auth store, WebAuthn verification, sessions, pairing codes
- `Cargo.toml` — `p256`, `ciborium`, `base64`, `getrandom`, `qrcode`
- `web/src` — auth status/sign-in/pairing screens, Devices dialog, 401 handling
- Credentials and sessions live in `~/.config/task-manager/web-auth.json` (0600), not in `tasks.db`, so sign-ins don't fire the task change watcher
- `AGENTS.md` — no change (task file format and task operations unchanged)
