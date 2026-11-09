//! Local-only consent-gated authorization and WebSocket relay prototype.
use axum::{
    extract::{Path, State},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, time::Duration};
use yt_session_core::{Error, SessionStore, Status};
mod relay;

#[derive(Clone)]
struct App { sessions: SessionStore, relay: relay::RelayHub }
#[derive(Serialize)]
struct NewSession { code: String, student_token: String, expires_in_seconds: u64 }
#[derive(Serialize)]
struct NewRequest { request_id: String, teacher_token: String }
#[derive(Serialize)]
struct SessionState { status: String, request_id: Option<String> }
#[derive(Deserialize)]
struct Decision { request_id: String }

fn bearer(headers: &HeaderMap) -> Result<&str, StatusCode> {
    headers.get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)
}
fn http_error(e: Error) -> StatusCode {
    match e {
        Error::InvalidCode => StatusCode::BAD_REQUEST,
        Error::Unauthorized => StatusCode::UNAUTHORIZED,
        Error::NotFound => StatusCode::NOT_FOUND,
        Error::Capacity => StatusCode::TOO_MANY_REQUESTS,
        Error::Expired => StatusCode::GONE,
        Error::InvalidRequest | Error::InvalidState => StatusCode::CONFLICT,
    }
}
fn state_name(s: Status) -> &'static str {
    match s {
        Status::Waiting => "waiting", Status::Requested => "requested",
        Status::Approved => "approved", Status::Denied => "denied",
        Status::Ended => "ended", Status::Expired => "expired",
    }
}
async fn create(State(app): State<App>) -> Result<Json<NewSession>, StatusCode> {
    let session = app.sessions.create(Duration::from_secs(900)).map_err(http_error)?;
    Ok(Json(NewSession { code: session.code, student_token: session.student_secret, expires_in_seconds: 900 }))
}
async fn request(Path(code): Path<String>, State(app): State<App>) -> Result<Json<NewRequest>, StatusCode> {
    let req = app.sessions.request(&code).map_err(http_error)?;
    Ok(Json(NewRequest { request_id: req.request_id, teacher_token: req.teacher_secret }))
}
async fn student_status(Path(code): Path<String>, State(app): State<App>, headers: HeaderMap) -> Result<Json<SessionState>, StatusCode> {
    let (status, request_id) = app.sessions.student_status(&code, bearer(&headers)?).map_err(http_error)?;
    Ok(Json(SessionState { status: state_name(status).to_string(), request_id }))
}
async fn teacher_status(Path(code): Path<String>, State(app): State<App>, headers: HeaderMap) -> Result<Json<SessionState>, StatusCode> {
    let status = app.sessions.teacher_status(&code, bearer(&headers)?).map_err(http_error)?;
    Ok(Json(SessionState { status: state_name(status).to_string(), request_id: None }))
}
async fn decide(
    Path((code, action)): Path<(String, String)>,
    State(app): State<App>,
    headers: HeaderMap,
    Json(body): Json<Decision>,
) -> Result<StatusCode, StatusCode> {
    let allow = match action.as_str() {
        "allow" => true, "deny" => false, _ => return Err(StatusCode::NOT_FOUND),
    };
    app.sessions.decide(&code, bearer(&headers)?, &body.request_id, allow).map_err(http_error)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn cancel(Path(code): Path<String>, State(app): State<App>, headers: HeaderMap) -> Result<StatusCode, StatusCode> {
    app.sessions.cancel_request(&code, bearer(&headers)?).map_err(http_error)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn teacher_disconnect(Path(code): Path<String>, State(app): State<App>, headers: HeaderMap) -> Result<StatusCode, StatusCode> {
    app.sessions.disconnect_teacher(&code, bearer(&headers)?).map_err(http_error)?;
    Ok(StatusCode::NO_CONTENT)
}
async fn stop(Path(code): Path<String>, State(app): State<App>, headers: HeaderMap) -> Result<StatusCode, StatusCode> {
    app.sessions.end(&code, bearer(&headers)?).map_err(http_error)?;
    Ok(StatusCode::NO_CONTENT)
}
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/v1/sessions", post(create))
        .route("/v1/sessions/:code/request", post(request))
        .route("/v1/sessions/:code/student", get(student_status))
        .route("/v1/sessions/:code/teacher", get(teacher_status))
        .route("/v1/sessions/:code/:action", post(decide))
        .route("/v1/sessions/:code/end", post(stop))
        .route("/v1/sessions/:code/cancel", post(cancel))
        .route("/v1/sessions/:code/disconnect", post(teacher_disconnect))
        .route("/v1/sessions/:code/ws/:role", get(relay::ws_handler))
        .with_state(App { sessions: SessionStore::new(), relay: relay::RelayHub::default() });
    // Localhost by default; a private Docker network may override this address.
    // Never expose the raw HTTP listener publicly without an HTTPS reverse proxy.
    let bind = std::env::var("YT_REMOTE_BIND")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_owned());
    let addr: SocketAddr = bind.parse().expect("invalid YT_REMOTE_BIND address");
    println!("YT Remote 2 development signaling/relay listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
