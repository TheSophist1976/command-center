## 1. Server: auth core (`src/web_auth.rs`)

- [x] 1.1 Add `p256`, `ciborium`, `base64`, `getrandom`, `qrcode` dependencies
- [x] 1.2 `AuthStore` persisted to `web-auth.json` (0600, atomic write): credentials and hashed sessions
- [x] 1.3 In-memory pairing codes (single-use, 10 min, 5 failures) and challenges (registration and sign-in, 5 min, single-use)
- [x] 1.4 Registration: build creation options; verify clientDataJSON, attestation object `authData`, extract COSE ES256 key
- [x] 1.5 Sign-in: build request options; verify assertion (rpIdHash, flags, challenge, origin, counter, signature)
- [x] 1.6 Sessions: create, validate with sliding expiry, delete; revoke credential deletes its sessions

## 2. Server: wiring (`src/server.rs`)

- [x] 2.1 Read `remote-host` at startup into `AppState`; print it in the startup banner
- [x] 2.2 Replace `validate_host` with classify + authorize middleware (local / remote / forbidden; 401 for unauthenticated remote API; Origin check on remote writes; local-only routes)
- [x] 2.3 `/api/auth/*` routes: status, pair, devices, revoke, register options/verify, login options/verify, logout
- [x] 2.4 Tests: classification, forged Host via proxy, end-to-end register + sign-in with a synthetic authenticator, negative cases, revoke

## 3. Web client

- [x] 3.1 `authStatus` on load; 401 from any API call returns to the sign-in screen
- [x] 3.2 Phone sign-in screen (Face ID button) and pairing screen (`?pair=CODE`)
- [x] 3.3 Desktop Devices dialog from Settings: setup instructions when `remote-host` is unset, Pair a phone (QR, link, code, countdown), device list with revoke
- [x] 3.4 Sign out in the phone drawer

## 4. Docs

- [x] 4.1 README: phone access via Tailscale (`tailscale serve`, `task config set remote-host`, pairing)
