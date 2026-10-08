## Context

`task serve` binds `127.0.0.1:4287` and its `validate_host` middleware only accepts `Host: localhost|127.0.0.1` (DNS-rebinding guard). The owner wants to use the web client from a phone over Tailscale. `tailscale serve --bg 4287` terminates HTTPS on `<machine>.<tailnet>.ts.net` and proxies to the loopback port, preserving the original Host header and adding `X-Forwarded-For/-Host/-Proto` and `Tailscale-User-*` headers.

## Goals / Non-goals

Goals: phone sign-in with Face ID; desktop keeps working with no sign-in; no new system dependencies in release builds.

Non-goals: multiple users or roles; password fallback; binding to non-loopback interfaces (HTTPS is the proxy's job); attestation verification (we request `attestation: "none"`).

## Decisions

**Classify by Host plus forwarding headers.** Local = loopback Host and none of `X-Forwarded-For`, `X-Forwarded-Host`, `Forwarded`, `Tailscale-User-Login`. A tailnet client that forges `Host: localhost` through the proxy still arrives with forwarding headers the proxy adds, so it's treated as remote, not local, and gets 403 (Host isn't `remote-host`).

**Pure-Rust ES256 instead of `webauthn-rs`.** `webauthn-rs` pulls OpenSSL, which the rustls-only release matrix avoids. Every platform passkey (iCloud Keychain, Google Password Manager, 1Password) supports ES256. Registration parses the CBOR attestation object only for `authData` and ignores the attestation statement. Assertions verify `rpIdHash`, UP+UV flags, `type`, `challenge`, `origin`, sign counter (when non-zero), and the ECDSA P-256 signature over `authData || SHA-256(clientDataJSON)`.

**Pairing codes gate registration.** Without them anyone on the tailnet could register their own passkey. Codes are 8 characters of Crockford base32 (40 bits), single-use, expire in 10 minutes, and die after 5 failed uses. Only local requests can create them.

**Storage in a separate JSON file.** `tasks.db` is watched and every write broadcasts a refresh to all clients; sign-ins shouldn't do that. Volume is tiny (a few credentials, a few sessions). Written atomically (temp file + rename), mode 0600. Challenges and pairing codes are in memory only — a restart invalidates in-flight ceremonies, which is fine.

**Sessions.** 32 random bytes, base64url in the cookie, SHA-256 hex on disk. 30-day sliding expiry; `last_seen` is persisted at most once an hour to limit writes. Revoking a credential deletes its sessions.

**CSRF.** Cookie is `SameSite=Strict`; remote non-GET requests also require `Origin: https://<remote-host>`.

**Remote-only restrictions.** `POST /api/update` (replaces the binary), `POST /api/notes/:slug/open` (spawns a desktop process) and device management stay local-only so a stolen phone session can't escalate.

## Risks / Trade-offs

- Hand-rolled WebAuthn verification → kept narrow (ES256, no attestation), covered by tests that drive a synthetic authenticator through register and sign-in, including bad origin, bad challenge, replayed challenge, missing UV, and tampered signature.
- Losing the phone → revoke its passkey from the desktop Devices dialog; sessions die with it.
- `remote-host` is read at startup → changing it needs a `task serve` restart (printed in the startup banner).
