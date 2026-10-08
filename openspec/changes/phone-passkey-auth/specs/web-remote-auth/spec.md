## ADDED Requirements

### Requirement: Request classification
The server SHALL classify every request as local, remote, or forbidden. A request is local when its Host hostname is `localhost` or `127.0.0.1` and it has none of the headers `X-Forwarded-For`, `X-Forwarded-Host`, `Forwarded`, `Tailscale-User-Login`. A request is remote when `remote-host` is configured and the Host hostname equals it (case-insensitive). All other requests SHALL receive 403.

#### Scenario: Desktop browser
- **WHEN** a request arrives with `Host: 127.0.0.1:4287` and no forwarding headers
- **THEN** it SHALL be served without sign-in

#### Scenario: Forged loopback Host through the proxy
- **WHEN** a request arrives with `Host: localhost` and an `X-Forwarded-For` header
- **THEN** it SHALL receive 403

### Requirement: Remote requests require a session
Remote requests to `/api/*` other than `/api/auth/status`, `/api/auth/register/*`, `/api/auth/login/*` and `/api/auth/logout` SHALL require a valid session cookie and SHALL receive 401 without one. Remote requests for the web client's static files SHALL be served without a session. Remote requests with a method other than GET or HEAD SHALL be rejected with 403 unless `Origin` equals `https://<remote-host>`.

#### Scenario: Unauthenticated remote API call
- **WHEN** a remote request without a session calls `GET /api/tasks`
- **THEN** the server SHALL respond 401

#### Scenario: Cross-origin remote write
- **WHEN** a remote request with a valid session sends `PATCH /api/tasks/1` with `Origin: https://evil.example`
- **THEN** the server SHALL respond 403

### Requirement: Local-only operations
Remote requests SHALL receive 403 for `POST /api/update`, `POST /api/notes/:slug/open`, `POST /api/auth/pair`, `GET /api/auth/devices` and `DELETE /api/auth/devices/:id`, even with a session.

#### Scenario: Update from phone
- **WHEN** a signed-in phone calls `POST /api/update`
- **THEN** the server SHALL respond 403

### Requirement: Pairing codes
A local request to `POST /api/auth/pair` SHALL create a pairing code of 8 Crockford base32 characters, valid for 10 minutes and usable once, and return the code, its expiry, the pairing URL `https://<remote-host>/?pair=<code>` and a QR code of that URL as SVG. The endpoint SHALL respond 409 when `remote-host` is not configured. A code SHALL be invalidated after 5 failed registration attempts.

#### Scenario: Pair without remote host
- **WHEN** `remote-host` is unset and the desktop calls `POST /api/auth/pair`
- **THEN** the server SHALL respond 409 with an error explaining how to set `remote-host`

### Requirement: Passkey registration
A remote client holding a valid pairing code SHALL be able to register a passkey: `POST /api/auth/register/options` returns WebAuthn creation options (rp id = `remote-host`, ES256 only, resident key and user verification required, attestation none). `POST /api/auth/register/verify` SHALL accept the credential only if `clientDataJSON.type` is `webauthn.create`, the challenge matches the one issued for that code, the origin is `https://<remote-host>`, the `authData` rpIdHash is SHA-256 of `remote-host`, the UP and UV flags are set, and the credential public key is an ES256 (P-256) COSE key. On success it SHALL consume the code, store the credential with the given device name, and start a session.

#### Scenario: Register with a valid code
- **WHEN** a phone registers with an unexpired code and a valid attestation
- **THEN** the credential SHALL be stored, the code consumed, and a session cookie set

#### Scenario: Reused code
- **WHEN** a second registration uses a code that was already consumed
- **THEN** the server SHALL respond 400

### Requirement: Passkey sign-in
`POST /api/auth/login/options` SHALL return WebAuthn request options with a fresh single-use challenge, rp id = `remote-host` and user verification required. `POST /api/auth/login/verify` SHALL accept an assertion only for a stored credential when the type is `webauthn.get`, the challenge was issued and not yet used, the origin and rpIdHash match, the UP and UV flags are set, the sign counter (when either value is non-zero) increased, and the ECDSA P-256 signature over `authData || SHA-256(clientDataJSON)` verifies. On success it SHALL start a session.

#### Scenario: Tampered signature
- **WHEN** an assertion's signature does not verify
- **THEN** the server SHALL respond 401 and SHALL NOT set a cookie

### Requirement: Sessions
A session SHALL be a random 32-byte token sent as cookie `cc_session` with `HttpOnly; Secure; SameSite=Strict; Path=/` and a 30-day sliding expiry. Only the SHA-256 of the token SHALL be stored. `POST /api/auth/logout` SHALL delete the session and clear the cookie.

#### Scenario: Expired session
- **WHEN** a remote request presents a session older than 30 days since last use
- **THEN** the server SHALL respond 401

### Requirement: Device management
A local request to `GET /api/auth/devices` SHALL list stored passkeys (id, name, created, last used). `DELETE /api/auth/devices/:id` SHALL remove the passkey and every session it created.

#### Scenario: Revoke a lost phone
- **WHEN** the desktop revokes a device
- **THEN** the phone's next remote API call SHALL receive 401
