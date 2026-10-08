//! Passkey (WebAuthn) sign-in for phones reaching `task serve` through a
//! proxy such as `tailscale serve`.
//!
//! Scope is deliberately narrow: one owner, ES256 passkeys only, user
//! verification required, attestation not checked (we ask for `none`).
//! Verification is pure Rust (`p256` + `ciborium`) so release builds stay
//! free of OpenSSL.
//!
//! Credentials and hashed session tokens persist to `web-auth.json` next to
//! the app config (mode 0600). Pairing codes and challenges live in memory
//! only, so a server restart cancels in-flight ceremonies.

use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use base64::Engine;
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SESSION_COOKIE: &str = "cc_session";
pub const SESSION_TTL_SECS: u64 = 30 * 86_400;
/// `last_seen` is written back to disk at most this often.
const LAST_SEEN_PERSIST_SECS: u64 = 3_600;
pub const PAIR_TTL_SECS: u64 = 600;
const PAIR_MAX_FAILURES: u32 = 5;
const CHALLENGE_TTL_SECS: u64 = 300;
/// Crockford base32: no I, L, O or U, so codes survive being read aloud.
const CODE_ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const COSE_ALG_ES256: i64 = -7;

const FLAG_UP: u8 = 0x01;
const FLAG_UV: u8 = 0x04;
const FLAG_AT: u8 = 0x40;

#[derive(Debug, PartialEq)]
pub enum AuthError {
    /// Malformed or out-of-date request (bad/expired pairing code, unknown challenge).
    BadRequest(String),
    /// The ceremony didn't verify.
    Unauthorized(String),
    Internal(String),
}

pub fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    getrandom::getrandom(&mut buf).expect("OS random number generator unavailable");
    buf
}

fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn hash_token(token: &str) -> String {
    hex(&sha256(token.as_bytes()))
}

/// Uppercases and maps the characters people confuse (O→0, I/L→1); drops spaces and dashes.
pub fn normalize_code(code: &str) -> String {
    code.chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Credential {
    /// Credential id, base64url.
    pub id: String,
    /// Uncompressed SEC1 P-256 point, base64url.
    pub public_key: String,
    pub sign_count: u32,
    pub name: String,
    pub created: u64,
    #[serde(default)]
    pub last_used: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Session {
    token_hash: String,
    credential_id: String,
    created: u64,
    last_seen: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct Persisted {
    /// WebAuthn user handle shared by every passkey (there is only one user).
    #[serde(default)]
    user_id: Option<String>,
    #[serde(default)]
    credentials: Vec<Credential>,
    #[serde(default)]
    sessions: Vec<Session>,
}

struct PairCode {
    expires: u64,
    failures: u32,
    /// Registration challenge issued for this code, base64url.
    challenge: Option<String>,
}

pub struct AuthStore {
    /// `None` keeps everything in memory (tests).
    path: Option<PathBuf>,
    data: Persisted,
    pair_codes: HashMap<String, PairCode>,
    /// Sign-in challenge (base64url) → expiry.
    login_challenges: HashMap<String, u64>,
}

// ---- WebAuthn client payloads (field names follow the JSON the browser produces) ----

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegistrationResponse {
    pub id: String,
    pub response: AttestationResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttestationResponse {
    #[serde(rename = "clientDataJSON")]
    pub client_data_json: String,
    pub attestation_object: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthenticationResponse {
    pub id: String,
    pub response: AssertionResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssertionResponse {
    #[serde(rename = "clientDataJSON")]
    pub client_data_json: String,
    pub authenticator_data: String,
    pub signature: String,
}

#[derive(Deserialize)]
struct ClientData {
    #[serde(rename = "type")]
    kind: String,
    challenge: String,
    origin: String,
    #[serde(rename = "crossOrigin", default)]
    cross_origin: bool,
}

struct AuthData {
    flags: u8,
    sign_count: u32,
    attested: Option<(Vec<u8>, VerifyingKey)>,
}

fn decode_b64(field: &str, value: &str) -> Result<Vec<u8>, AuthError> {
    B64.decode(value.trim_end_matches('='))
        .map_err(|_| AuthError::BadRequest(format!("{} is not base64url", field)))
}

/// Checks type, origin and cross-origin; returns the challenge it was signed over.
fn check_client_data(raw: &[u8], expected_type: &str, origin: &str) -> Result<String, AuthError> {
    let cd: ClientData = serde_json::from_slice(raw)
        .map_err(|_| AuthError::BadRequest("clientDataJSON is not valid JSON".into()))?;
    if cd.kind != expected_type {
        return Err(AuthError::Unauthorized(format!("unexpected ceremony type {}", cd.kind)));
    }
    if cd.origin != origin {
        return Err(AuthError::Unauthorized(format!("unexpected origin {}", cd.origin)));
    }
    if cd.cross_origin {
        return Err(AuthError::Unauthorized("cross-origin ceremony".into()));
    }
    Ok(cd.challenge.trim_end_matches('=').to_string())
}

fn parse_cose_es256(value: &ciborium::Value) -> Result<VerifyingKey, AuthError> {
    let bad = |m: &str| AuthError::Unauthorized(format!("credential key: {}", m));
    let map = value.as_map().ok_or_else(|| bad("not a map"))?;
    let get = |k: i64| {
        map.iter()
            .find(|(key, _)| key.as_integer().map(i128::from) == Some(k as i128))
            .map(|(_, v)| v)
    };
    let int = |k: i64| get(k).and_then(|v| v.as_integer()).map(i128::from);
    if int(1) != Some(2) {
        return Err(bad("key type is not EC2"));
    }
    if int(3) != Some(COSE_ALG_ES256 as i128) {
        return Err(bad("algorithm is not ES256"));
    }
    if int(-1) != Some(1) {
        return Err(bad("curve is not P-256"));
    }
    let x = get(-2).and_then(|v| v.as_bytes()).ok_or_else(|| bad("missing x"))?;
    let y = get(-3).and_then(|v| v.as_bytes()).ok_or_else(|| bad("missing y"))?;
    if x.len() != 32 || y.len() != 32 {
        return Err(bad("bad coordinate length"));
    }
    let mut point = Vec::with_capacity(65);
    point.push(0x04);
    point.extend_from_slice(x);
    point.extend_from_slice(y);
    VerifyingKey::from_sec1_bytes(&point).map_err(|_| bad("point is not on P-256"))
}

/// Parses authenticator data, checking the rpIdHash and the UP + UV flags.
fn parse_auth_data(bytes: &[u8], rp_id: &str, expect_attested: bool) -> Result<AuthData, AuthError> {
    if bytes.len() < 37 {
        return Err(AuthError::Unauthorized("authenticator data too short".into()));
    }
    if bytes[..32] != sha256(rp_id.as_bytes()) {
        return Err(AuthError::Unauthorized("passkey belongs to a different site".into()));
    }
    let flags = bytes[32];
    if flags & FLAG_UP == 0 || flags & FLAG_UV == 0 {
        return Err(AuthError::Unauthorized("user verification (Face ID / passcode) is required".into()));
    }
    let sign_count = u32::from_be_bytes([bytes[33], bytes[34], bytes[35], bytes[36]]);
    let attested = if expect_attested {
        if flags & FLAG_AT == 0 || bytes.len() < 55 {
            return Err(AuthError::Unauthorized("no attested credential data".into()));
        }
        let len = u16::from_be_bytes([bytes[53], bytes[54]]) as usize;
        let id_end = 55 + len;
        if bytes.len() < id_end {
            return Err(AuthError::Unauthorized("credential id truncated".into()));
        }
        let cred_id = bytes[55..id_end].to_vec();
        let mut rest = &bytes[id_end..];
        let cose: ciborium::Value = ciborium::from_reader(&mut rest)
            .map_err(|_| AuthError::Unauthorized("credential key is not CBOR".into()))?;
        Some((cred_id, parse_cose_es256(&cose)?))
    } else {
        None
    };
    Ok(AuthData { flags, sign_count, attested })
}

/// Pulls `authData` out of a CBOR attestation object. The attestation
/// statement is ignored: we request `attestation: "none"`.
fn attestation_auth_data(attestation_object: &[u8]) -> Result<Vec<u8>, AuthError> {
    let value: ciborium::Value = ciborium::from_reader(attestation_object)
        .map_err(|_| AuthError::BadRequest("attestationObject is not CBOR".into()))?;
    value
        .as_map()
        .and_then(|m| m.iter().find(|(k, _)| k.as_text() == Some("authData")))
        .and_then(|(_, v)| v.as_bytes().cloned())
        .ok_or_else(|| AuthError::BadRequest("attestationObject has no authData".into()))
}

impl AuthStore {
    pub fn in_memory() -> Self {
        AuthStore { path: None, data: Persisted::default(), pair_codes: HashMap::new(), login_challenges: HashMap::new() }
    }

    /// Loads `path`, or starts empty if it doesn't exist. A file that exists
    /// but can't be parsed is an error rather than silently discarded.
    pub fn load(path: PathBuf) -> Result<Self, String> {
        let data = match fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).map_err(|e| format!("{} is corrupt: {}", path.display(), e))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Persisted::default(),
            Err(e) => return Err(format!("Failed to read {}: {}", path.display(), e)),
        };
        Ok(AuthStore { path: Some(path), data, pair_codes: HashMap::new(), login_challenges: HashMap::new() })
    }

    pub fn default_path() -> Option<PathBuf> {
        crate::config::config_path().and_then(|p| p.parent().map(|d| d.join("web-auth.json")))
    }

    fn save(&self) -> Result<(), AuthError> {
        let Some(path) = &self.path else { return Ok(()) };
        write_private(path, &serde_json::to_vec_pretty(&self.data).map_err(|e| AuthError::Internal(e.to_string()))?)
            .map_err(AuthError::Internal)
    }

    fn prune(&mut self, now: u64) {
        self.pair_codes.retain(|_, c| c.expires > now);
        self.login_challenges.retain(|_, exp| *exp > now);
    }

    // ---- pairing ----

    /// Creates a single-use pairing code; returns it with its expiry (unix seconds).
    pub fn create_pair_code(&mut self) -> (String, u64) {
        let now = now_secs();
        self.prune(now);
        let code: String = random_bytes::<8>().iter().map(|b| CODE_ALPHABET[(*b as usize) % 32] as char).collect();
        let expires = now + PAIR_TTL_SECS;
        self.pair_codes.insert(code.clone(), PairCode { expires, failures: 0, challenge: None });
        (code, expires)
    }

    fn user_id(&mut self) -> Result<String, AuthError> {
        if let Some(id) = &self.data.user_id {
            return Ok(id.clone());
        }
        let id = B64.encode(random_bytes::<16>());
        self.data.user_id = Some(id.clone());
        self.save()?;
        Ok(id)
    }

    /// WebAuthn `PublicKeyCredentialCreationOptions` (JSON, binary fields base64url).
    pub fn register_options(&mut self, rp_id: &str, code: &str) -> Result<serde_json::Value, AuthError> {
        let now = now_secs();
        self.prune(now);
        let code = normalize_code(code);
        if !self.pair_codes.contains_key(&code) {
            return Err(AuthError::BadRequest("Pairing code is invalid or expired. Create a new one on your computer.".into()));
        }
        let user_id = self.user_id()?;
        let challenge = B64.encode(random_bytes::<32>());
        self.pair_codes.get_mut(&code).expect("checked above").challenge = Some(challenge.clone());
        let exclude: Vec<_> = self
            .data
            .credentials
            .iter()
            .map(|c| serde_json::json!({ "type": "public-key", "id": c.id }))
            .collect();
        Ok(serde_json::json!({
            "challenge": challenge,
            "rp": { "id": rp_id, "name": "command center" },
            "user": { "id": user_id, "name": "command-center", "displayName": "command center" },
            "pubKeyCredParams": [{ "type": "public-key", "alg": COSE_ALG_ES256 }],
            "authenticatorSelection": { "residentKey": "required", "requireResidentKey": true, "userVerification": "required" },
            "attestation": "none",
            "timeout": CHALLENGE_TTL_SECS * 1000,
            "excludeCredentials": exclude,
        }))
    }

    /// Verifies a registration; on success consumes the code, stores the
    /// passkey and returns a new session token.
    pub fn register_verify(
        &mut self,
        rp_id: &str,
        origin: &str,
        code: &str,
        name: &str,
        resp: &RegistrationResponse,
    ) -> Result<String, AuthError> {
        let now = now_secs();
        self.prune(now);
        let code = normalize_code(code);
        let Some(pending) = self.pair_codes.get(&code) else {
            return Err(AuthError::BadRequest("Pairing code is invalid or expired. Create a new one on your computer.".into()));
        };
        let Some(expected_challenge) = pending.challenge.clone() else {
            return Err(AuthError::BadRequest("Start pairing again: no challenge was issued for this code.".into()));
        };

        match Self::verify_registration(rp_id, origin, &expected_challenge, resp) {
            Ok((cred_id, key)) => {
                self.pair_codes.remove(&code);
                let id = B64.encode(&cred_id);
                self.data.credentials.retain(|c| c.id != id);
                let name = name.trim();
                self.data.credentials.push(Credential {
                    id: id.clone(),
                    public_key: B64.encode(key.to_encoded_point(false).as_bytes()),
                    sign_count: 0,
                    name: if name.is_empty() { "Phone".to_string() } else { name.chars().take(60).collect() },
                    created: now,
                    last_used: Some(now),
                });
                self.start_session(&id, now)
            }
            Err(e) => {
                let pending = self.pair_codes.get_mut(&code).expect("checked above");
                pending.failures += 1;
                pending.challenge = None;
                if pending.failures >= PAIR_MAX_FAILURES {
                    self.pair_codes.remove(&code);
                }
                Err(e)
            }
        }
    }

    fn verify_registration(
        rp_id: &str,
        origin: &str,
        expected_challenge: &str,
        resp: &RegistrationResponse,
    ) -> Result<(Vec<u8>, VerifyingKey), AuthError> {
        let client_data = decode_b64("clientDataJSON", &resp.response.client_data_json)?;
        let challenge = check_client_data(&client_data, "webauthn.create", origin)?;
        if challenge != expected_challenge {
            return Err(AuthError::Unauthorized("challenge mismatch".into()));
        }
        let att = decode_b64("attestationObject", &resp.response.attestation_object)?;
        let auth_data = parse_auth_data(&attestation_auth_data(&att)?, rp_id, true)?;
        let (cred_id, key) = auth_data.attested.expect("expect_attested");
        if B64.encode(&cred_id) != resp.id.trim_end_matches('=') {
            return Err(AuthError::Unauthorized("credential id mismatch".into()));
        }
        Ok((cred_id, key))
    }

    // ---- sign-in ----

    /// WebAuthn `PublicKeyCredentialRequestOptions` for a discoverable passkey.
    pub fn login_options(&mut self, rp_id: &str) -> serde_json::Value {
        let now = now_secs();
        self.prune(now);
        let challenge = B64.encode(random_bytes::<32>());
        self.login_challenges.insert(challenge.clone(), now + CHALLENGE_TTL_SECS);
        serde_json::json!({
            "challenge": challenge,
            "rpId": rp_id,
            "userVerification": "required",
            "timeout": CHALLENGE_TTL_SECS * 1000,
            "allowCredentials": [],
        })
    }

    pub fn login_verify(&mut self, rp_id: &str, origin: &str, resp: &AuthenticationResponse) -> Result<String, AuthError> {
        let now = now_secs();
        self.prune(now);
        let id = resp.id.trim_end_matches('=').to_string();
        let idx = self
            .data
            .credentials
            .iter()
            .position(|c| c.id == id)
            .ok_or_else(|| AuthError::Unauthorized("This passkey isn't paired with this computer.".into()))?;

        let client_data = decode_b64("clientDataJSON", &resp.response.client_data_json)?;
        let challenge = check_client_data(&client_data, "webauthn.get", origin)?;
        // Single use, whether or not the rest verifies.
        if self.login_challenges.remove(&challenge).is_none() {
            return Err(AuthError::Unauthorized("sign-in challenge is unknown or expired".into()));
        }
        let auth_bytes = decode_b64("authenticatorData", &resp.response.authenticator_data)?;
        let auth_data = parse_auth_data(&auth_bytes, rp_id, false)?;
        debug_assert!(auth_data.flags & FLAG_UV != 0);

        let cred = &self.data.credentials[idx];
        let key_bytes = decode_b64("public key", &cred.public_key).map_err(|_| AuthError::Internal("stored key is corrupt".into()))?;
        let key = VerifyingKey::from_sec1_bytes(&key_bytes).map_err(|_| AuthError::Internal("stored key is corrupt".into()))?;
        let sig_bytes = decode_b64("signature", &resp.response.signature)?;
        let sig = Signature::from_der(&sig_bytes).map_err(|_| AuthError::Unauthorized("signature is malformed".into()))?;
        let mut signed = auth_bytes.clone();
        signed.extend_from_slice(&sha256(&client_data));
        key.verify(&signed, &sig).map_err(|_| AuthError::Unauthorized("signature does not verify".into()))?;

        // Synced passkeys report 0; only enforce the counter once it's in use.
        if (auth_data.sign_count != 0 || cred.sign_count != 0) && auth_data.sign_count <= cred.sign_count {
            return Err(AuthError::Unauthorized("sign counter went backwards — possible cloned passkey".into()));
        }

        let cred = &mut self.data.credentials[idx];
        cred.sign_count = auth_data.sign_count;
        cred.last_used = Some(now);
        self.start_session(&id, now)
    }

    // ---- sessions ----

    fn start_session(&mut self, credential_id: &str, now: u64) -> Result<String, AuthError> {
        let token = B64.encode(random_bytes::<32>());
        self.data.sessions.retain(|s| s.last_seen + SESSION_TTL_SECS > now);
        self.data.sessions.push(Session { token_hash: hash_token(&token), credential_id: credential_id.to_string(), created: now, last_seen: now });
        self.save()?;
        Ok(token)
    }

    /// True if `token` names a live session; extends its sliding expiry.
    pub fn validate_session(&mut self, token: &str) -> bool {
        self.validate_session_at(token, now_secs())
    }

    fn validate_session_at(&mut self, token: &str, now: u64) -> bool {
        let hash = hash_token(token);
        let Some(session) = self.data.sessions.iter_mut().find(|s| s.token_hash == hash) else { return false };
        if session.last_seen + SESSION_TTL_SECS <= now {
            return false;
        }
        let persist = now.saturating_sub(session.last_seen) >= LAST_SEEN_PERSIST_SECS;
        session.last_seen = now;
        if persist {
            if let Err(e) = self.save() {
                eprintln!("task serve: failed to save web-auth.json: {:?}", e);
            }
        }
        true
    }

    pub fn end_session(&mut self, token: &str) -> Result<(), AuthError> {
        let hash = hash_token(token);
        let before = self.data.sessions.len();
        self.data.sessions.retain(|s| s.token_hash != hash);
        if self.data.sessions.len() != before {
            self.save()?;
        }
        Ok(())
    }

    // ---- devices ----

    pub fn devices(&self) -> Vec<Credential> {
        self.data.credentials.clone()
    }

    /// Removes a passkey and every session it started. Returns false if unknown.
    pub fn revoke(&mut self, credential_id: &str) -> Result<bool, AuthError> {
        let before = self.data.credentials.len();
        self.data.credentials.retain(|c| c.id != credential_id);
        if self.data.credentials.len() == before {
            return Ok(false);
        }
        self.data.sessions.retain(|s| s.credential_id != credential_id);
        self.save()?;
        Ok(true)
    }
}

/// Writes `bytes` to `path` atomically with mode 0600 on Unix.
fn write_private(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create {}: {}", parent.display(), e))?;
    }
    let tmp = path.with_extension("json.tmp");
    {
        use std::io::Write;
        let mut opts = fs::OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp).map_err(|e| format!("Failed to write {}: {}", tmp.display(), e))?;
        f.write_all(bytes).and_then(|_| f.sync_all()).map_err(|e| format!("Failed to write {}: {}", tmp.display(), e))?;
    }
    fs::rename(&tmp, path).map_err(|e| format!("Failed to replace {}: {}", path.display(), e))
}

/// Remote-access settings plus the auth store, shared by every request.
pub struct RemoteAuth {
    /// Hostname the phone uses (e.g. `desk.tail1234.ts.net`), from the
    /// `remote-host` config key. `None` disables remote access entirely.
    pub remote_host: Option<String>,
    pub store: Mutex<AuthStore>,
}

impl RemoteAuth {
    /// Local-only: no remote host, nothing persisted.
    pub fn disabled() -> Self {
        RemoteAuth { remote_host: None, store: Mutex::new(AuthStore::in_memory()) }
    }

    pub fn new(remote_host: Option<String>, store: AuthStore) -> Self {
        let remote_host = remote_host
            .map(|h| normalize_host(&h))
            .filter(|h| !h.is_empty());
        RemoteAuth { remote_host, store: Mutex::new(store) }
    }

    /// Reads `remote-host` from config and the store from its default path.
    pub fn from_config() -> Result<Self, String> {
        let host = crate::config::read_config_value("remote-host");
        let store = match AuthStore::default_path() {
            Some(path) => AuthStore::load(path)?,
            None => AuthStore::in_memory(),
        };
        Ok(Self::new(host, store))
    }

    pub fn origin(&self) -> Option<String> {
        self.remote_host.as_ref().map(|h| format!("https://{}", h))
    }

    /// `https://<remote-host>/?pair=<code>` and an SVG QR code of it.
    pub fn pairing_link(&self, code: &str) -> Option<(String, String)> {
        let url = format!("{}/?pair={}", self.origin()?, code);
        let svg = qrcode::QrCode::new(url.as_bytes())
            .ok()?
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(192, 192)
            .quiet_zone(true)
            .dark_color(qrcode::render::svg::Color("#2E3440"))
            .light_color(qrcode::render::svg::Color("#ECEFF4"))
            .build();
        Some((url, svg))
    }
}

/// Lowercase, without scheme, path or port: `https://Desk.ts.net:443/` → `desk.ts.net`.
pub fn normalize_host(raw: &str) -> String {
    let s = raw.trim();
    let s = s.strip_prefix("https://").or_else(|| s.strip_prefix("http://")).unwrap_or(s);
    let s = s.split('/').next().unwrap_or("");
    let s = s.split(':').next().unwrap_or("");
    s.to_ascii_lowercase()
}

#[cfg(test)]
pub mod test_authenticator {
    //! A software passkey for driving real ceremonies in tests.
    use super::*;
    use p256::ecdsa::signature::Signer;
    use p256::ecdsa::SigningKey;

    pub struct SoftPasskey {
        pub key: SigningKey,
        pub cred_id: Vec<u8>,
        pub counter: u32,
    }

    impl SoftPasskey {
        pub fn new() -> Self {
            SoftPasskey { key: SigningKey::random(&mut rand_core::OsRng), cred_id: random_bytes::<16>().to_vec(), counter: 0 }
        }

        pub fn id(&self) -> String {
            B64.encode(&self.cred_id)
        }

        fn client_data(kind: &str, challenge: &str, origin: &str) -> Vec<u8> {
            serde_json::to_vec(&serde_json::json!({ "type": kind, "challenge": challenge, "origin": origin, "crossOrigin": false })).unwrap()
        }

        pub fn auth_data(rp_id: &str, flags: u8, counter: u32) -> Vec<u8> {
            let mut a = sha256(rp_id.as_bytes()).to_vec();
            a.push(flags);
            a.extend_from_slice(&counter.to_be_bytes());
            a
        }

        pub fn register(&self, rp_id: &str, origin: &str, challenge: &str, flags: u8) -> serde_json::Value {
            let point = self.key.verifying_key().to_encoded_point(false);
            let cose = ciborium::Value::Map(vec![
                (1.into(), 2.into()),
                (3.into(), (-7).into()),
                ((-1).into(), 1.into()),
                ((-2).into(), ciborium::Value::Bytes(point.x().unwrap().to_vec())),
                ((-3).into(), ciborium::Value::Bytes(point.y().unwrap().to_vec())),
            ]);
            let mut auth = Self::auth_data(rp_id, flags | FLAG_AT, 0);
            auth.extend_from_slice(&[0u8; 16]);
            auth.extend_from_slice(&(self.cred_id.len() as u16).to_be_bytes());
            auth.extend_from_slice(&self.cred_id);
            ciborium::into_writer(&cose, &mut auth).unwrap();
            let att = ciborium::Value::Map(vec![
                ("fmt".into(), "none".into()),
                ("attStmt".into(), ciborium::Value::Map(vec![])),
                ("authData".into(), ciborium::Value::Bytes(auth)),
            ]);
            let mut att_bytes = Vec::new();
            ciborium::into_writer(&att, &mut att_bytes).unwrap();
            serde_json::json!({
                "id": self.id(), "rawId": self.id(), "type": "public-key",
                "response": {
                    "clientDataJSON": B64.encode(Self::client_data("webauthn.create", challenge, origin)),
                    "attestationObject": B64.encode(att_bytes),
                }
            })
        }

        pub fn sign_in(&mut self, rp_id: &str, origin: &str, challenge: &str, flags: u8) -> serde_json::Value {
            self.counter += 1;
            let auth = Self::auth_data(rp_id, flags, self.counter);
            let cd = Self::client_data("webauthn.get", challenge, origin);
            let mut msg = auth.clone();
            msg.extend_from_slice(&sha256(&cd));
            let sig: Signature = self.key.sign(&msg);
            serde_json::json!({
                "id": self.id(), "rawId": self.id(), "type": "public-key",
                "response": {
                    "clientDataJSON": B64.encode(cd),
                    "authenticatorData": B64.encode(auth),
                    "signature": B64.encode(sig.to_der().as_bytes()),
                    "userHandle": null,
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_authenticator::SoftPasskey;
    use super::*;

    const RP: &str = "desk.tail1234.ts.net";
    const ORIGIN: &str = "https://desk.tail1234.ts.net";
    const OK_FLAGS: u8 = FLAG_UP | FLAG_UV;

    fn register(store: &mut AuthStore, pk: &SoftPasskey) -> String {
        let (code, _) = store.create_pair_code();
        let opts = store.register_options(RP, &code).unwrap();
        let resp: RegistrationResponse =
            serde_json::from_value(pk.register(RP, ORIGIN, opts["challenge"].as_str().unwrap(), OK_FLAGS)).unwrap();
        store.register_verify(RP, ORIGIN, &code, "iPhone", &resp).unwrap()
    }

    fn sign_in(store: &mut AuthStore, pk: &mut SoftPasskey) -> Result<String, AuthError> {
        let opts = store.login_options(RP);
        let resp: AuthenticationResponse =
            serde_json::from_value(pk.sign_in(RP, ORIGIN, opts["challenge"].as_str().unwrap(), OK_FLAGS)).unwrap();
        store.login_verify(RP, ORIGIN, &resp)
    }

    #[test]
    fn register_then_sign_in() {
        let mut store = AuthStore::in_memory();
        let mut pk = SoftPasskey::new();
        let token = register(&mut store, &pk);
        assert!(store.validate_session(&token));
        assert_eq!(store.devices().len(), 1);
        assert_eq!(store.devices()[0].name, "iPhone");
        let token2 = sign_in(&mut store, &mut pk).unwrap();
        assert!(store.validate_session(&token2));
        assert!(!store.validate_session("not-a-token"));
    }

    #[test]
    fn pairing_code_is_single_use() {
        let mut store = AuthStore::in_memory();
        let pk = SoftPasskey::new();
        let (code, _) = store.create_pair_code();
        let opts = store.register_options(RP, &code).unwrap();
        let resp: RegistrationResponse =
            serde_json::from_value(pk.register(RP, ORIGIN, opts["challenge"].as_str().unwrap(), OK_FLAGS)).unwrap();
        store.register_verify(RP, ORIGIN, &code, "a", &resp).unwrap();
        assert!(matches!(store.register_options(RP, &code), Err(AuthError::BadRequest(_))));
        assert!(matches!(store.register_verify(RP, ORIGIN, &code, "a", &resp), Err(AuthError::BadRequest(_))));
    }

    #[test]
    fn pairing_code_normalizes_confusable_characters() {
        assert_eq!(normalize_code(" ab-cd o il "), "ABCD011");
    }

    #[test]
    fn unknown_pairing_code_rejected() {
        let mut store = AuthStore::in_memory();
        assert!(matches!(store.register_options(RP, "ZZZZZZZZ"), Err(AuthError::BadRequest(_))));
    }

    #[test]
    fn registration_rejects_wrong_origin_challenge_and_missing_uv() {
        let mut store = AuthStore::in_memory();
        let pk = SoftPasskey::new();
        let (code, _) = store.create_pair_code();
        for (origin, use_real_challenge, flags) in [
            ("https://evil.example", true, OK_FLAGS),
            (ORIGIN, false, OK_FLAGS),
            (ORIGIN, true, FLAG_UP),
        ] {
            let opts = store.register_options(RP, &code).unwrap();
            let challenge = if use_real_challenge { opts["challenge"].as_str().unwrap().to_string() } else { B64.encode([7u8; 32]) };
            let resp: RegistrationResponse = serde_json::from_value(pk.register(RP, origin, &challenge, flags)).unwrap();
            assert!(matches!(store.register_verify(RP, ORIGIN, &code, "x", &resp), Err(AuthError::Unauthorized(_))));
        }
        assert!(store.devices().is_empty());
    }

    #[test]
    fn registration_rejects_other_rp_id() {
        let mut store = AuthStore::in_memory();
        let pk = SoftPasskey::new();
        let (code, _) = store.create_pair_code();
        let opts = store.register_options(RP, &code).unwrap();
        let resp: RegistrationResponse =
            serde_json::from_value(pk.register("evil.example", ORIGIN, opts["challenge"].as_str().unwrap(), OK_FLAGS)).unwrap();
        assert!(matches!(store.register_verify(RP, ORIGIN, &code, "x", &resp), Err(AuthError::Unauthorized(_))));
    }

    #[test]
    fn code_dies_after_repeated_failures() {
        let mut store = AuthStore::in_memory();
        let pk = SoftPasskey::new();
        let (code, _) = store.create_pair_code();
        for _ in 0..PAIR_MAX_FAILURES {
            let opts = store.register_options(RP, &code).unwrap();
            let resp: RegistrationResponse =
                serde_json::from_value(pk.register(RP, "https://evil.example", opts["challenge"].as_str().unwrap(), OK_FLAGS)).unwrap();
            let _ = store.register_verify(RP, ORIGIN, &code, "x", &resp);
        }
        assert!(matches!(store.register_options(RP, &code), Err(AuthError::BadRequest(_))));
    }

    #[test]
    fn sign_in_rejects_tampered_signature_and_replay() {
        let mut store = AuthStore::in_memory();
        let mut pk = SoftPasskey::new();
        register(&mut store, &pk);

        let opts = store.login_options(RP);
        let challenge = opts["challenge"].as_str().unwrap().to_string();
        let mut v = pk.sign_in(RP, ORIGIN, &challenge, OK_FLAGS);
        let good: AuthenticationResponse = serde_json::from_value(v.clone()).unwrap();
        // Flip the counter inside authenticatorData so the signature no longer covers it.
        let mut auth = B64.decode(v["response"]["authenticatorData"].as_str().unwrap()).unwrap();
        auth[36] ^= 0xff;
        v["response"]["authenticatorData"] = B64.encode(auth).into();
        let bad: AuthenticationResponse = serde_json::from_value(v).unwrap();
        assert!(matches!(store.login_verify(RP, ORIGIN, &bad), Err(AuthError::Unauthorized(_))));
        // The challenge was consumed by the failed attempt, so even the genuine response is now a replay.
        assert!(matches!(store.login_verify(RP, ORIGIN, &good), Err(AuthError::Unauthorized(_))));
    }

    #[test]
    fn sign_in_rejects_unknown_passkey_and_counter_rollback() {
        let mut store = AuthStore::in_memory();
        let mut pk = SoftPasskey::new();
        register(&mut store, &pk);
        let mut stranger = SoftPasskey::new();
        assert!(sign_in(&mut store, &mut stranger).is_err());

        sign_in(&mut store, &mut pk).unwrap();
        sign_in(&mut store, &mut pk).unwrap();
        pk.counter = 0; // a clone that missed the last two sign-ins
        assert!(sign_in(&mut store, &mut pk).is_err());
    }

    #[test]
    fn revoke_ends_sessions() {
        let mut store = AuthStore::in_memory();
        let pk = SoftPasskey::new();
        let token = register(&mut store, &pk);
        assert!(store.revoke(&pk.id()).unwrap());
        assert!(!store.validate_session(&token));
        assert!(!store.revoke(&pk.id()).unwrap());
    }

    #[test]
    fn sessions_expire_after_ttl_of_inactivity() {
        let mut store = AuthStore::in_memory();
        let pk = SoftPasskey::new();
        let token = register(&mut store, &pk);
        let start = now_secs();
        assert!(store.validate_session_at(&token, start + SESSION_TTL_SECS - 10));
        // Sliding: still alive a full TTL after the last use…
        assert!(store.validate_session_at(&token, start + 2 * SESSION_TTL_SECS - 20));
        // …but not after a full TTL of inactivity.
        assert!(!store.validate_session_at(&token, start + 3 * SESSION_TTL_SECS));
        store.end_session(&token).unwrap();
        assert!(!store.validate_session(&token));
    }

    #[test]
    fn store_persists_and_reloads() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("web-auth.json");
        let mut store = AuthStore::load(path.clone()).unwrap();
        let pk = SoftPasskey::new();
        let token = register(&mut store, &pk);
        let mut reloaded = AuthStore::load(path.clone()).unwrap();
        assert!(reloaded.validate_session(&token));
        assert_eq!(reloaded.devices()[0].id, pk.id());
        let raw = fs::read_to_string(&path).unwrap();
        assert!(!raw.contains(&token), "raw session tokens must not be written to disk");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }

    #[test]
    fn corrupt_store_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("web-auth.json");
        fs::write(&path, "{nope").unwrap();
        assert!(AuthStore::load(path).is_err());
    }

    #[test]
    fn normalize_host_strips_scheme_port_and_path() {
        assert_eq!(normalize_host("https://Desk.Tail1234.ts.net:443/x"), "desk.tail1234.ts.net");
        assert_eq!(normalize_host("desk.ts.net"), "desk.ts.net");
    }

    #[test]
    fn pairing_link_includes_code_and_qr() {
        let remote = RemoteAuth::new(Some("https://Desk.ts.net/".into()), AuthStore::in_memory());
        let (url, svg) = remote.pairing_link("ABCD2345").unwrap();
        assert_eq!(url, "https://desk.ts.net/?pair=ABCD2345");
        assert!(svg.contains("<svg"));
        assert!(RemoteAuth::disabled().pairing_link("X").is_none());
    }
}
