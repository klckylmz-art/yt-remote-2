//! Consent-gated bidirectional relay for video frames and input commands.
//! Transport is intentionally bound to localhost until TLS and deployment are configured.
//! This is not a full remote desktop implementation.
use std::{collections::HashMap, sync::{Arc, Mutex, Weak}, time::Duration};
use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::Deserialize;
use tokio::sync::broadcast;
use yt_session_core::Status;
use crate::{bearer, App};

const FRAME_LIMIT: usize = 3 * 1024 * 1024;
const INPUT_LIMIT: usize = 4096;
const CHANNEL_CAPACITY: usize = 4;

#[derive(Clone)]
pub(crate) struct RelayHub {
    inner: Arc<Mutex<HashMap<String, Weak<Channels>>>>,
}
impl Default for RelayHub {
    fn default() -> Self { Self { inner: Arc::new(Mutex::new(HashMap::new())) } }
}
#[derive(Clone)]
struct Channels {
    frames: broadcast::Sender<Vec<u8>>,
    inputs: broadcast::Sender<String>,
}
impl RelayHub {
    fn channels(&self, code: &str) -> Arc<Channels> {
        let mut inner = self.inner.lock().expect("relay mutex poisoned");
        // Dead sessions must not retain queued JPEG frames or unbounded channel entries.
        inner.retain(|_, weak| weak.strong_count() > 0);
        if let Some(existing) = inner.get(code).and_then(Weak::upgrade) {
            return existing;
        }
        let (frames, _) = broadcast::channel(CHANNEL_CAPACITY);
        let (inputs, _) = broadcast::channel(CHANNEL_CAPACITY);
        let channels = Arc::new(Channels { frames, inputs });
        inner.insert(code.to_owned(), Arc::downgrade(&channels));
        channels
    }
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum InputCommand {
    Move { x: f64, y: f64 },
    Button { x: f64, y: f64, button: String, pressed: bool },
    Scroll { delta: i32 },
    Key { name: String, pressed: bool },
}
fn valid_input(raw: &str) -> bool {
    if raw.len() > INPUT_LIMIT { return false; }
    match serde_json::from_str::<InputCommand>(raw) {
        Ok(InputCommand::Move { x, y }) =>
            x.is_finite() && y.is_finite() && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y),
        Ok(InputCommand::Button { x, y, button, pressed }) => {
            let _ = pressed;
            x.is_finite() && y.is_finite() && (0.0..=1.0).contains(&x)
                && (0.0..=1.0).contains(&y)
                && matches!(button.as_str(), "left" | "right" | "middle")
        }
        Ok(InputCommand::Scroll { delta }) => (-5..=5).contains(&delta),
        Ok(InputCommand::Key { name, pressed }) => {
            let _ = pressed;
            !name.is_empty() && name.len() <= 32
                && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        }
        Err(_) => false,
    }
}
// Called only after SessionStore has validated the code and bearer token.
fn canonical_channel_key(code: &str) -> String {
    code.chars()
        .filter(|c| *c != '-')
        .flat_map(char::to_uppercase)
        .collect()
}
fn approved(app: &App, code: &str, token: &str, student: bool) -> bool {
    if student {
        matches!(app.sessions.student_status(code, token), Ok((Status::Approved, _)))
    } else {
        matches!(app.sessions.teacher_status(code, token), Ok(Status::Approved))
    }
}
pub(crate) async fn ws_handler(
    ws: WebSocketUpgrade,
    Path((code, role)): Path<(String, String)>,
    State(app): State<App>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let student = match role.as_str() {
        "student" => true, "teacher" => false, _ => return Err(StatusCode::NOT_FOUND),
    };
    let token = bearer(&headers)?;
    if !approved(&app, &code, token, student) { return Err(StatusCode::FORBIDDEN); }
    // The bearer is never included in an invitation and is never logged.
    let token = token.to_owned();
    // SessionStore accepts lowercase and hyphen-free codes. All such aliases
    // must share the same relay channels, otherwise approved peers can connect
    // successfully yet never exchange video or input.
    let channel_key = canonical_channel_key(&code);
    let channels = app.relay.channels(&channel_key);
    Ok(ws.max_message_size(FRAME_LIMIT + 1024)
        .max_frame_size(FRAME_LIMIT + 1024)
        .on_upgrade(move |socket| relay_socket(socket, app, code, token, student, channels)))
}
async fn relay_socket(
    mut socket: WebSocket,
    app: App,
    code: String,
    token: String,
    student: bool,
    channels: Arc<Channels>,
) {
    let mut frames = channels.frames.subscribe();
    let mut inputs = channels.inputs.subscribe();
    let mut gate = tokio::time::interval(Duration::from_millis(200));
    loop {
        tokio::select! {
            _ = gate.tick() => {
                if !approved(&app, &code, &token, student) {
                    let _ = socket.send(Message::Close(None)).await;
                    return;
                }
            }
            incoming = socket.recv() => {
                if !approved(&app, &code, &token, student) { break; }
                match incoming {
                    Some(Ok(Message::Binary(data))) if student && data.len() <= FRAME_LIMIT => {
                        // JPEG marker check: do not turn arbitrary binaries into file transfer.
                        if data.starts_with(&[0xff, 0xd8, 0xff]) && data.ends_with(&[0xff, 0xd9]) {
                            let _ = channels.frames.send(data);
                        }
                    }
                    Some(Ok(Message::Text(text))) if !student && valid_input(&text) => {
                        let _ = channels.inputs.send(text);
                    }
                    Some(Ok(Message::Ping(_))) | Some(Ok(Message::Pong(_))) => {}
                    Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                    _ => {}
                }
            }
            msg = frames.recv(), if !student => {
                match msg {
                    Ok(data) if approved(&app, &code, &token, student) => {
                        if socket.send(Message::Binary(data)).await.is_err() { break; }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    _ => {}
                }
            }
            msg = inputs.recv(), if student => {
                match msg {
                    Ok(cmd) if approved(&app, &code, &token, student) => {
                        if socket.send(Message::Text(cmd)).await.is_err() { break; }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                    _ => {}
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}
#[cfg(test)]
mod tests {
    use super::valid_input;
    #[test]
    fn rejects_file_transfer_and_unsafe_commands() {
        assert!(valid_input(r#"{"kind":"move","x":0.3,"y":0.9}"#));
        assert!(valid_input(r#"{"kind":"key","name":"enter","pressed":true}"#));
        assert!(!valid_input(r#"{"kind":"file","path":"/tmp/a"}"#));
        assert!(!valid_input(r#"{"kind":"move","x":50,"y":0.2}"#));
        assert!(!valid_input(r#"{"kind":"key","name":"ctrl+c","pressed":true}"#));
    }
    #[test]
    fn approved_code_aliases_use_identical_relay_channels() {
        use super::{canonical_channel_key, RelayHub};
        let hub = RelayHub::default();
        let canonical = "ABCD-EFGH-JKLM-NPQR";
        let lower = "abcd-efgh-jklm-npqr";
        let compact = "ABCDEFGHJKLMNPQR";
        assert_eq!(canonical_channel_key(canonical), canonical_channel_key(lower));
        assert_eq!(canonical_channel_key(canonical), canonical_channel_key(compact));
        let first = hub.channels(&canonical_channel_key(canonical));
        let second = hub.channels(&canonical_channel_key(lower));
        let third = hub.channels(&canonical_channel_key(compact));
        assert!(std::sync::Arc::ptr_eq(&first, &second));
        assert!(std::sync::Arc::ptr_eq(&first, &third));
        let mut receiver = second.frames.subscribe();
        first.frames.send(vec![0xff, 0xd8, 0xff, 0xd9]).unwrap();
        assert_eq!(receiver.try_recv().unwrap(), vec![0xff, 0xd8, 0xff, 0xd9]);
    }
    #[test]
    fn relay_refuses_access_until_approval_and_after_revocation() {
        use super::{approved, App, RelayHub};
        use std::time::Duration;
        use yt_session_core::SessionStore;
        let sessions = SessionStore::new();
        let app = App { sessions: sessions.clone(), relay: RelayHub::default() };
        let student = sessions.create(Duration::from_secs(120)).unwrap();
        let teacher = sessions.request(&student.code).unwrap();
        assert!(!approved(&app, &student.code, &teacher.teacher_secret, false));
        assert!(!approved(&app, &student.code, &student.student_secret, true));
        sessions.decide(&student.code, &student.student_secret, &teacher.request_id, true).unwrap();
        assert!(approved(&app, &student.code, &teacher.teacher_secret, false));
        assert!(approved(&app, &student.code, &student.student_secret, true));
        sessions.end(&student.code, &student.student_secret).unwrap();
        assert!(!approved(&app, &student.code, &teacher.teacher_secret, false));
        assert!(!approved(&app, &student.code, &student.student_secret, true));
    }
}
