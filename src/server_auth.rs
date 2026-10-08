//! HTTP side of remote sign-in: classifies each request as local or remote,
//! enforces sessions and restrictions for remote requests, and serves
//! `/api/auth/*`. The ceremonies themselves live in `crate::web_auth`.

use std::sync::Arc;

use axum::extract::{Path, Request, State};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Extension, Json, Router};
use serde::{Deserialize, Serialize};

use crate::server::AppState;
use crate::web_auth::{AuthError, AuthenticationResponse, RegistrationResponse, SESSION_COOKIE, SESSION_TTL_SECS};

/// How a request reached the server. Inserted as a request extension by [`guard`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Access {
    /// Desktop browser on this machine: trusted, no sign-in.
    Local,
    /// Through the proxy on `remote-host`: needs a session.
    Remote,
}

/// Headers a reverse proxy (e.g. `tailscale serve`) adds. Their presence means
/// the request did not come straight from a browser on this machine, whatever
/// its Host header claims.
const FORWARDING_HEADERS: [&str; 4] = ["x-forwarded-for", "x-forwarded-host", "forwarded", "tailscale-user-login"];

/// Paths a remote client may call before signing in.
const PUBLIC_AUTH_PATHS: [&str; 6] = [
    "/api/auth/status",
    "/api/auth/register/options",
    "/api/auth/register/verify",
    "/api/auth/login/options",
    "/api/auth/login/verify",
    "/api/auth/logout",
];

pub fn classify(headers: &HeaderMap, remote_host: Option<&str>) -> Option<Access> {
    let host = headers.get(header::HOST).and_then(|v| v.to_str().ok())?;
    let hostname = host.split(':').next().unwrap_or("").to_ascii_lowercase();
    let forwarded = FORWARDING_HEADERS.iter().any(|h| headers.contains_key(*h));
    if (hostname == "127.0.0.1" || hostname == "localhost") && !forwarded {
        return Some(Access::Local);
    }
    match remote_host {
        Some(remote) if hostname == remote => Some(Access::Remote),
        _ => None,
    }
}

/// Operations that stay on the desktop even for a signed-in phone: they
/// replace the binary, spawn local processes, or mint/revoke passkeys.
fn local_only(method: &Method, path: &str) -> bool {
    (method == Method::POST && path == "/api/update")
        || (method == Method::POST && path.starts_with("/api/notes/") && path.ends_with("/open"))
        || path == "/api/auth/pair"
        || path == "/api/auth/devices"
        || path.starts_with("/api/auth/devices/")
}

pub fn session_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(k, _)| *k == SESSION_COOKIE)
        .map(|(_, v)| v.to_string())
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (status, Json(serde_json::json!({ "error": message }))).into_response()
}

/// Replaces the old Host-only check. Local requests pass as before; remote
/// requests must target `remote-host`, carry a matching `Origin` on writes,
/// and hold a session for anything under `/api/` except sign-in itself.
pub async fn guard(State(state): State<Arc<AppState>>, mut request: Request, next: Next) -> Response {
    let remote = &state.remote;
    let Some(access) = classify(request.headers(), remote.remote_host.as_deref()) else {
        return (StatusCode::FORBIDDEN, "Forbidden").into_response();
    };
    if access == Access::Remote {
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        if local_only(&method, &path) {
            return json_error(StatusCode::FORBIDDEN, "Only available on the computer running command center");
        }
        if method != Method::GET && method != Method::HEAD {
            let origin = request.headers().get(header::ORIGIN).and_then(|v| v.to_str().ok());
            if origin != remote.origin().as_deref() {
                return json_error(StatusCode::FORBIDDEN, "Cross-origin request refused");
            }
        }
        if path.starts_with("/api/") && !PUBLIC_AUTH_PATHS.contains(&path.as_str()) {
            let signed_in = session_token(request.headers())
                .is_some_and(|t| remote.store.lock().unwrap().validate_session(&t));
            if !signed_in {
                return json_error(StatusCode::UNAUTHORIZED, "Sign in required");
            }
        }
    }
    request.extensions_mut().insert(access);
    next.run(request).await
}

fn auth_error(e: AuthError) -> Response {
    match e {
        AuthError::BadRequest(m) => json_error(StatusCode::BAD_REQUEST, &m),
        AuthError::Unauthorized(m) => json_error(StatusCode::UNAUTHORIZED, &m),
        AuthError::Internal(m) => {
            eprintln!("task serve: auth error: {}", m);
            json_error(StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
        }
    }
}

fn no_remote_host() -> Response {
    json_error(
        StatusCode::CONFLICT,
        "Remote access is off. Run `tailscale serve --bg 4287`, then `task config set remote-host <name>.ts.net`, and restart `task serve`.",
    )
}

fn with_session_cookie(token: &str) -> Response {
    let cookie = format!("{}={}; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age={}", SESSION_COOKIE, token, SESSION_TTL_SECS);
    ([(header::SET_COOKIE, cookie)], Json(serde_json::json!({ "ok": true }))).into_response()
}

#[derive(Serialize)]
struct StatusResponse {
    local: bool,
    authenticated: bool,
    remote_host: Option<String>,
}

async fn status(State(state): State<Arc<AppState>>, Extension(access): Extension<Access>, headers: HeaderMap) -> Json<StatusResponse> {
    let authenticated = access == Access::Local
        || session_token(&headers).is_some_and(|t| state.remote.store.lock().unwrap().validate_session(&t));
    Json(StatusResponse { local: access == Access::Local, authenticated, remote_host: state.remote.remote_host.clone() })
}

async fn pair(State(state): State<Arc<AppState>>) -> Response {
    if state.remote.remote_host.is_none() {
        return no_remote_host();
    }
    let (code, expires_at) = state.remote.store.lock().unwrap().create_pair_code();
    let Some((url, qr_svg)) = state.remote.pairing_link(&code) else {
        return json_error(StatusCode::INTERNAL_SERVER_ERROR, "could not build the pairing link");
    };
    Json(serde_json::json!({ "code": code, "url": url, "qr_svg": qr_svg, "expires_at": expires_at })).into_response()
}

#[derive(Serialize)]
struct Device {
    id: String,
    name: String,
    created: u64,
    last_used: Option<u64>,
}

async fn list_devices(State(state): State<Arc<AppState>>) -> Json<Vec<Device>> {
    let devices = state.remote.store.lock().unwrap().devices();
    Json(devices.into_iter().map(|c| Device { id: c.id, name: c.name, created: c.created, last_used: c.last_used }).collect())
}

async fn revoke_device(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> Response {
    match state.remote.store.lock().unwrap().revoke(&id) {
        Ok(true) => StatusCode::NO_CONTENT.into_response(),
        Ok(false) => json_error(StatusCode::NOT_FOUND, "no such device"),
        Err(e) => auth_error(e),
    }
}

#[derive(Deserialize)]
struct RegisterOptionsBody {
    code: String,
}

async fn register_options(State(state): State<Arc<AppState>>, Json(body): Json<RegisterOptionsBody>) -> Response {
    let Some(rp_id) = state.remote.remote_host.clone() else { return no_remote_host() };
    match state.remote.store.lock().unwrap().register_options(&rp_id, &body.code) {
        Ok(options) => Json(options).into_response(),
        Err(e) => auth_error(e),
    }
}

#[derive(Deserialize)]
struct RegisterVerifyBody {
    code: String,
    #[serde(default)]
    name: String,
    credential: RegistrationResponse,
}

async fn register_verify(State(state): State<Arc<AppState>>, Json(body): Json<RegisterVerifyBody>) -> Response {
    let (Some(rp_id), Some(origin)) = (state.remote.remote_host.clone(), state.remote.origin()) else { return no_remote_host() };
    let result = state.remote.store.lock().unwrap().register_verify(&rp_id, &origin, &body.code, &body.name, &body.credential);
    match result {
        Ok(token) => with_session_cookie(&token),
        Err(e) => auth_error(e),
    }
}

async fn login_options(State(state): State<Arc<AppState>>) -> Response {
    let Some(rp_id) = state.remote.remote_host.clone() else { return no_remote_host() };
    Json(state.remote.store.lock().unwrap().login_options(&rp_id)).into_response()
}

#[derive(Deserialize)]
struct LoginVerifyBody {
    credential: AuthenticationResponse,
}

async fn login_verify(State(state): State<Arc<AppState>>, Json(body): Json<LoginVerifyBody>) -> Response {
    let (Some(rp_id), Some(origin)) = (state.remote.remote_host.clone(), state.remote.origin()) else { return no_remote_host() };
    let result = state.remote.store.lock().unwrap().login_verify(&rp_id, &origin, &body.credential);
    match result {
        Ok(token) => with_session_cookie(&token),
        Err(e) => auth_error(e),
    }
}

async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Some(token) = session_token(&headers) {
        if let Err(e) = state.remote.store.lock().unwrap().end_session(&token) {
            return auth_error(e);
        }
    }
    let clear = format!("{}=; Path=/; HttpOnly; Secure; SameSite=Strict; Max-Age=0", SESSION_COOKIE);
    ([(header::SET_COOKIE, clear)], StatusCode::NO_CONTENT).into_response()
}

pub fn auth_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/auth/status", get(status))
        .route("/api/auth/pair", post(pair))
        .route("/api/auth/devices", get(list_devices))
        .route("/api/auth/devices/:id", delete(revoke_device))
        .route("/api/auth/register/options", post(register_options))
        .route("/api/auth/register/verify", post(register_verify))
        .route("/api/auth/login/options", post(login_options))
        .route("/api/auth/login/verify", post(login_verify))
        .route("/api/auth/logout", post(logout))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web_auth::test_authenticator::SoftPasskey;
    use crate::web_auth::{AuthStore, RemoteAuth};
    use axum::body::Body;
    use tower::ServiceExt;

    const HOST: &str = "desk.tail1234.ts.net";
    const ORIGIN: &str = "https://desk.tail1234.ts.net";
    const UP_UV: u8 = 0x01 | 0x04;

    fn app() -> (tempfile::TempDir, Router) {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            db_path: dir.path().join("tasks.db"),
            notes_dir: dir.path().join("Notes"),
            write_lock: Arc::new(tokio::sync::Mutex::new(())),
            change_tx: tokio::sync::broadcast::channel(16).0,
            update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)),
            remote: Arc::new(RemoteAuth::new(Some(HOST.into()), AuthStore::in_memory())),
        };
        (dir, crate::server::router(state))
    }

    /// A request as `tailscale serve` would deliver it from the phone.
    fn remote(method: &str, uri: &str, cookie: Option<&str>, body: Option<serde_json::Value>) -> axum::http::Request<Body> {
        let mut b = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("host", HOST)
            .header("x-forwarded-for", "100.101.102.103")
            .header("x-forwarded-proto", "https")
            .header("origin", ORIGIN)
            .header("content-type", "application/json");
        if let Some(c) = cookie {
            b = b.header("cookie", format!("other=1; {}={}", SESSION_COOKIE, c));
        }
        b.body(body.map(|v| Body::from(v.to_string())).unwrap_or_else(Body::empty)).unwrap()
    }

    fn local(method: &str, uri: &str) -> axum::http::Request<Body> {
        axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "127.0.0.1:4287")
            .header("x-command-center", "update")
            .body(Body::empty())
            .unwrap()
    }

    async fn json(resp: Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }

    fn cookie_token(resp: &Response) -> String {
        let set = resp.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(set.contains("HttpOnly") && set.contains("Secure") && set.contains("SameSite=Strict"));
        set.split(';').next().unwrap().split_once('=').unwrap().1.to_string()
    }

    /// Pairs a phone from the desktop and returns its session token.
    async fn pair_phone(app: &Router, pk: &SoftPasskey) -> String {
        let pair = json(app.clone().oneshot(local("POST", "/api/auth/pair")).await.unwrap()).await;
        let code = pair["code"].as_str().unwrap().to_string();
        assert_eq!(pair["url"], format!("{}/?pair={}", ORIGIN, code));
        let opts = json(app.clone().oneshot(remote("POST", "/api/auth/register/options", None, Some(serde_json::json!({ "code": code })))).await.unwrap()).await;
        assert_eq!(opts["rp"]["id"], HOST);
        let cred = pk.register(HOST, ORIGIN, opts["challenge"].as_str().unwrap(), UP_UV);
        let resp = app
            .clone()
            .oneshot(remote("POST", "/api/auth/register/verify", None, Some(serde_json::json!({ "code": code, "name": "iPhone", "credential": cred }))))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        cookie_token(&resp)
    }

    #[test]
    fn classify_local_remote_and_forbidden() {
        let h = |pairs: &[(&str, &str)]| {
            let mut m = HeaderMap::new();
            for (k, v) in pairs {
                m.insert(axum::http::HeaderName::from_bytes(k.as_bytes()).unwrap(), v.parse().unwrap());
            }
            m
        };
        assert_eq!(classify(&h(&[("host", "localhost:4287")]), Some(HOST)), Some(Access::Local));
        assert_eq!(classify(&h(&[("host", "127.0.0.1")]), None), Some(Access::Local));
        assert_eq!(classify(&h(&[("host", "Desk.Tail1234.ts.net"), ("x-forwarded-for", "1.2.3.4")]), Some(HOST)), Some(Access::Remote));
        // A tailnet peer forging `Host: localhost` still arrives with the proxy's forwarding headers.
        assert_eq!(classify(&h(&[("host", "localhost"), ("x-forwarded-for", "1.2.3.4")]), Some(HOST)), None);
        assert_eq!(classify(&h(&[("host", "127.0.0.1"), ("tailscale-user-login", "a@b")]), Some(HOST)), None);
        assert_eq!(classify(&h(&[("host", HOST)]), None), None);
        assert_eq!(classify(&h(&[("host", "evil.example")]), Some(HOST)), None);
        assert_eq!(classify(&h(&[]), Some(HOST)), None);
    }

    #[tokio::test]
    async fn remote_needs_session_but_gets_static_and_status() {
        let (_d, app) = app();
        let r = app.clone().oneshot(remote("GET", "/api/tasks", None, None)).await.unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        let r = app.clone().oneshot(remote("GET", "/api/events", Some("bogus"), None)).await.unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        let status = json(app.clone().oneshot(remote("GET", "/api/auth/status", None, None)).await.unwrap()).await;
        assert_eq!(status["local"], false);
        assert_eq!(status["authenticated"], false);
        // Local requests are untouched.
        let r = app.clone().oneshot(local("GET", "/api/tasks")).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let status = json(app.clone().oneshot(local("GET", "/api/auth/status")).await.unwrap()).await;
        assert_eq!(status["local"], true);
        assert_eq!(status["authenticated"], true);
    }

    #[tokio::test]
    async fn pair_sign_in_use_and_revoke() {
        let (_d, app) = app();
        let mut pk = SoftPasskey::new();
        let token = pair_phone(&app, &pk).await;

        let r = app.clone().oneshot(remote("GET", "/api/tasks", Some(&token), None)).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let r = app.clone().oneshot(remote("POST", "/api/tasks", Some(&token), Some(serde_json::json!({ "title": "from phone" })))).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);

        // Fresh sign-in on a new browser session.
        let opts = json(app.clone().oneshot(remote("POST", "/api/auth/login/options", None, None)).await.unwrap()).await;
        let cred = pk.sign_in(HOST, ORIGIN, opts["challenge"].as_str().unwrap(), UP_UV);
        let r = app.clone().oneshot(remote("POST", "/api/auth/login/verify", None, Some(serde_json::json!({ "credential": cred })))).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let token2 = cookie_token(&r);

        let devices = json(app.clone().oneshot(local("GET", "/api/auth/devices")).await.unwrap()).await;
        assert_eq!(devices[0]["name"], "iPhone");
        let r = app.clone().oneshot(local("DELETE", &format!("/api/auth/devices/{}", pk.id()))).await.unwrap();
        assert_eq!(r.status(), StatusCode::NO_CONTENT);
        for t in [&token, &token2] {
            let r = app.clone().oneshot(remote("GET", "/api/tasks", Some(t), None)).await.unwrap();
            assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        }
    }

    #[tokio::test]
    async fn remote_restrictions_hold_even_when_signed_in() {
        let (_d, app) = app();
        let pk = SoftPasskey::new();
        let token = pair_phone(&app, &pk).await;
        for (method, uri) in [
            ("POST", "/api/update"),
            ("POST", "/api/notes/some-note/open"),
            ("POST", "/api/auth/pair"),
            ("GET", "/api/auth/devices"),
            ("DELETE", "/api/auth/devices/abc"),
        ] {
            let r = app.clone().oneshot(remote(method, uri, Some(&token), None)).await.unwrap();
            assert_eq!(r.status(), StatusCode::FORBIDDEN, "{} {}", method, uri);
        }
        let mut cross = remote("POST", "/api/tasks", Some(&token), Some(serde_json::json!({ "title": "x" })));
        cross.headers_mut().insert(header::ORIGIN, "https://evil.example".parse().unwrap());
        assert_eq!(app.clone().oneshot(cross).await.unwrap().status(), StatusCode::FORBIDDEN);
        let mut no_origin = remote("POST", "/api/tasks", Some(&token), Some(serde_json::json!({ "title": "x" })));
        no_origin.headers_mut().remove(header::ORIGIN);
        assert_eq!(app.clone().oneshot(no_origin).await.unwrap().status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn logout_ends_the_session() {
        let (_d, app) = app();
        let pk = SoftPasskey::new();
        let token = pair_phone(&app, &pk).await;
        let r = app.clone().oneshot(remote("POST", "/api/auth/logout", Some(&token), None)).await.unwrap();
        assert_eq!(r.status(), StatusCode::NO_CONTENT);
        assert!(r.headers().get(header::SET_COOKIE).unwrap().to_str().unwrap().contains("Max-Age=0"));
        let r = app.clone().oneshot(remote("GET", "/api/tasks", Some(&token), None)).await.unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn pairing_needs_remote_host() {
        let dir = tempfile::tempdir().unwrap();
        let state = AppState {
            db_path: dir.path().join("tasks.db"),
            notes_dir: dir.path().join("Notes"),
            write_lock: Arc::new(tokio::sync::Mutex::new(())),
            change_tx: tokio::sync::broadcast::channel(16).0,
            update: Arc::new(crate::update_state::UpdateState::for_tests("http://127.0.0.1:1", false)),
            remote: Arc::new(RemoteAuth::disabled()),
        };
        let app = crate::server::router(state);
        let r = app.clone().oneshot(local("POST", "/api/auth/pair")).await.unwrap();
        assert_eq!(r.status(), StatusCode::CONFLICT);
        // And with no remote host every non-local request is refused outright.
        let r = app.oneshot(remote("GET", "/api/auth/status", None, None)).await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
    }
}
