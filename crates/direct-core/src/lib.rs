//! Initial authenticated WSS direct prototype. No capture or input injection occurs here.
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use futures_util::{SinkExt, StreamExt};
use rand::RngCore;
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering::SeqCst},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Notify,
    time::timeout,
};
use tokio_tungstenite::{
    tungstenite::{
        handshake::server::{Request, Response},
        http::{self, StatusCode},
        protocol::WebSocketConfig,
        Message,
    },
    WebSocketStream,
};

pub const MAX_MESSAGE: usize = 3 * 1024 * 1024;
const INPUT_CAPACITY: usize = 64;
const INVITE_SECONDS: u64 = 600;
const HANDSHAKE: Duration = Duration::from_secs(5);
const IDLE: Duration = Duration::from_secs(30);
const LIFETIME: Duration = Duration::from_secs(60 * 60);

#[derive(Default)]
pub struct Stats {
    pub last_rtt_ms: AtomicU64,
    pub sent: AtomicU64,
    pub received: AtomicU64,
    pub dropped: AtomicU64,
}
#[derive(Debug, Clone)]
pub enum Event {
    Invitation(String),
    Connected,
    Approved,
    Ended(String),
    Error(String),
}
#[derive(Debug, Clone)]
pub struct Frame {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub jpeg: Vec<u8>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputCommand {
    Move {
        x: f64,
        y: f64,
    },
    Button {
        button: String,
        pressed: bool,
        x: f64,
        y: f64,
    },
    Scroll {
        delta: i32,
    },
    Key {
        name: String,
        pressed: bool,
    },
}
impl InputCommand {
    pub fn valid(&self) -> bool {
        let point = |x: f64, y: f64| {
            x.is_finite() && y.is_finite() && (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)
        };
        match self {
            Self::Move { x, y } => point(*x, *y),
            Self::Button { button, x, y, .. } => {
                point(*x, *y) && matches!(button.as_str(), "left" | "right" | "middle")
            }
            Self::Scroll { delta } => (-5..=5).contains(delta),
            Self::Key { name, .. } => {
                (name.chars().count() == 1 && name.chars().all(|c| !c.is_control()))
                    || matches!(
                        name.as_str(),
                        "shift"
                            | "control"
                            | "alt"
                            | "command"
                            | "meta"
                            | "enter"
                            | "escape"
                            | "tab"
                            | "space"
                            | "backspace"
                            | "delete"
                            | "left"
                            | "right"
                            | "up"
                            | "down"
                            | "home"
                            | "end"
                            | "pageup"
                            | "pagedown"
                            | "capslock"
                            | "f1"
                            | "f2"
                            | "f3"
                            | "f4"
                            | "f5"
                            | "f6"
                            | "f7"
                            | "f8"
                            | "f9"
                            | "f10"
                            | "f11"
                            | "f12"
                    )
            }
        }
    }
}
impl Frame {
    fn valid(&self) -> bool {
        self.width > 0
            && self.height > 0
            && u64::from(self.width) * u64::from(self.height) <= 1920 * 1080
            && self.width <= 3840
            && self.height <= 3840
            && self.jpeg.len() + 21 <= MAX_MESSAGE
    }
}
#[derive(Clone, Copy, PartialEq)]
enum Role {
    Teacher,
    Student,
}
#[derive(Default)]
struct Mailboxes {
    events: VecDeque<Event>,
    inputs: VecDeque<InputCommand>,
    commands: VecDeque<Wire>,
    incoming: Option<Frame>,
    outgoing: Option<Frame>,
}
struct Shared {
    active: Arc<AtomicBool>,
    approved: Arc<AtomicBool>,
    stats: Arc<Stats>,
    boxes: Mutex<Mailboxes>,
    wake: Notify,
}
impl Shared {
    fn revoke(&self) {
        let mut b = self.boxes.lock().unwrap();
        self.approved.store(false, SeqCst);
        self.active.store(false, SeqCst);
        b.inputs.clear();
        b.commands.clear();
        b.incoming = None;
        b.outgoing = None;
        self.wake.notify_one();
    }
    fn event(&self, e: Event) {
        let mut b = self.boxes.lock().unwrap();
        if b.events.len() < 32 {
            b.events.push_back(e);
        }
    }
}
pub struct Session {
    pub active: Arc<AtomicBool>,
    pub approved: Arc<AtomicBool>,
    pub stats: Arc<Stats>,
    shared: Arc<Shared>,
    role: Role,
}
impl Session {
    /// Serialize a short native input operation against revocation. Do not
    /// perform capture, encoding or network I/O while holding this gate.
    pub fn with_consent<T>(&self, operation: impl FnOnce() -> T) -> Option<T> {
        let _guard = self.shared.boxes.lock().unwrap();
        if self.active.load(SeqCst) && self.approved.load(SeqCst) {
            Some(operation())
        } else {
            None
        }
    }
    fn new(role: Role) -> Self {
        let shared = Arc::new(Shared {
            active: Arc::new(AtomicBool::new(true)),
            approved: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(Stats::default()),
            boxes: Mutex::new(Mailboxes::default()),
            wake: Notify::new(),
        });
        Self {
            active: shared.active.clone(),
            approved: shared.approved.clone(),
            stats: shared.stats.clone(),
            shared,
            role,
        }
    }
    pub fn try_event(&self) -> Option<Event> {
        self.shared.boxes.lock().unwrap().events.pop_front()
    }
    pub fn take_frame(&self) -> Option<Frame> {
        let mut b = self.shared.boxes.lock().unwrap();
        if self.active.load(SeqCst) && self.approved.load(SeqCst) {
            b.incoming.take()
        } else {
            b.incoming = None;
            None
        }
    }
    pub fn try_input(&self) -> Option<InputCommand> {
        let mut b = self.shared.boxes.lock().unwrap();
        if self.active.load(SeqCst) && self.approved.load(SeqCst) {
            b.inputs.pop_front()
        } else {
            b.inputs.clear();
            None
        }
    }
    pub fn allow(&self) -> bool {
        if self.role != Role::Student {
            return false;
        }
        let mut b = self.shared.boxes.lock().unwrap();
        if !self.active.load(SeqCst)
            || self.approved.load(SeqCst)
            || b.commands.len() >= INPUT_CAPACITY
        {
            return false;
        }
        self.approved.store(true, SeqCst);
        b.commands.push_back(Wire::Approved);
        drop(b);
        self.shared.wake.notify_one();
        true
    }
    pub fn deny(&self) {
        self.stop();
    }
    pub fn stop(&self) {
        self.shared.revoke();
    }
    pub fn send_input(&self, input: InputCommand) -> bool {
        if self.role != Role::Teacher || !input.valid() {
            return false;
        }
        let mut b = self.shared.boxes.lock().unwrap();
        if !self.active.load(SeqCst) || !self.approved.load(SeqCst) {
            return false;
        }
        if b.commands.len() >= INPUT_CAPACITY {
            drop(b);
            self.shared.revoke();
            return false;
        }
        b.commands.push_back(Wire::Input(input));
        drop(b);
        self.shared.wake.notify_one();
        true
    }
    pub fn publish_frame(&self, frame: Frame) -> bool {
        if self.role != Role::Student || !frame.valid() {
            return false;
        }
        let mut b = self.shared.boxes.lock().unwrap();
        if !self.active.load(SeqCst) || !self.approved.load(SeqCst) {
            return false;
        }
        if b.outgoing.replace(frame).is_some() {
            self.stats.dropped.fetch_add(1, SeqCst);
        }
        drop(b);
        self.shared.wake.notify_one();
        true
    }
    pub fn ack_frame(&self, sequence: u64) {
        let mut b = self.shared.boxes.lock().unwrap();
        if self.role == Role::Teacher
            && self.active.load(SeqCst)
            && self.approved.load(SeqCst)
            && b.commands.len() < INPUT_CAPACITY
        {
            b.commands.push_back(Wire::Ack(sequence));
            drop(b);
            self.shared.wake.notify_one();
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
    }
}
#[derive(Clone, Copy, Default)]
pub struct TeacherOptions {
    loopback: bool,
}
impl TeacherOptions {
    pub fn loopback() -> Self {
        Self { loopback: true }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Invitation {
    version: u8,
    endpoint: SocketAddr,
    cert: String,
    token: String,
    expires: u64,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn parse_invitation(code: &str) -> Result<Invitation, String> {
    if code.len() > 8192 {
        return Err("Invitation too long".into());
    }
    let payload = code
        .trim()
        .strip_prefix("YTR3.")
        .ok_or("Invalid invitation prefix")?;
    let raw = URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|_| "Invalid invitation")?;
    let i: Invitation = serde_json::from_slice(&raw).map_err(|_| "Invalid invitation")?;
    if i.version != 3
        || i.expires <= now()
        || i.expires > now() + INVITE_SECONDS
        || i.endpoint.port() == 0
        || i.endpoint.ip().is_unspecified()
        || i.endpoint.ip().is_multicast()
        || i.token.len() != 43
    {
        return Err("Invalid or expired invitation".into());
    }
    if URL_SAFE_NO_PAD
        .decode(&i.token)
        .map_err(|_| "Invalid token")?
        .len()
        != 32
    {
        return Err("Invalid token".into());
    }
    let cert = URL_SAFE_NO_PAD
        .decode(&i.cert)
        .map_err(|_| "Invalid certificate")?;
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from(cert))
        .map_err(|_| "Invalid certificate")?;
    Ok(i)
}
pub fn start_student(code: &str) -> Result<Session, String> {
    let invitation = parse_invitation(code)?;
    let session = Session::new(Role::Student);
    let shared = session.shared.clone();
    launch(shared, move |s| async move {
        student_worker(s, invitation).await
    });
    Ok(session)
}
pub fn start_teacher(options: TeacherOptions) -> Session {
    let session = Session::new(Role::Teacher);
    let shared = session.shared.clone();
    launch(
        shared,
        move |s| async move { teacher_worker(s, options).await },
    );
    session
}
fn launch<F, Fut>(s: Arc<Shared>, f: F)
where
    F: FnOnce(Arc<Shared>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + 'static,
{
    let failed = s.clone();
    if std::thread::Builder::new()
        .name("direct-wss".into())
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|_| "Transport runtime failed".to_string())
                    .and_then(|rt| rt.block_on(f(s.clone())))
            }))
            .unwrap_or_else(|_| Err("The transport worker stopped unexpectedly".into()));
            s.revoke();
            s.event(match result {
                Ok(()) => Event::Ended("Session ended".into()),
                Err(e) => Event::Error(e),
            });
        })
        .is_err()
    {
        failed.revoke();
        failed.event(Event::Error("Cannot start transport worker".into()));
    }
}

struct Lease {
    gateway: igd_next::Gateway,
    port: u16,
}
impl Drop for Lease {
    fn drop(&mut self) {
        let _ = self
            .gateway
            .remove_port(igd_next::PortMappingProtocol::TCP, self.port);
    }
}
fn mapping(port: u16, active: &AtomicBool) -> Result<(SocketAddr, Lease), String> {
    let gateway = igd_next::search_gateway(igd_next::SearchOptions {
        timeout: Some(Duration::from_secs(3)),
        ..Default::default()
    })
    .map_err(|_| "No direct UPnP gateway available; this network is unsupported")?;
    if !active.load(SeqCst) {
        return Err("Invitation cancelled".into());
    }
    let route = std::net::UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .map_err(|_| "Cannot discover local route")?;
    route
        .connect(gateway.addr)
        .map_err(|_| "Cannot discover gateway route")?;
    let IpAddr::V4(local) = route
        .local_addr()
        .map_err(|_| "Cannot discover local address")?
        .ip()
    else {
        return Err("IPv4 gateway required".into());
    };
    let external = gateway
        .get_external_ip()
        .map_err(|_| "Cannot discover external address")?;
    if !active.load(SeqCst) {
        return Err("Invitation cancelled".into());
    }
    gateway
        .add_port(
            igd_next::PortMappingProtocol::TCP,
            port,
            SocketAddrV4::new(local, port).into(),
            600,
            "YT Remote temporary session",
        )
        .map_err(|_| "Cannot create temporary direct mapping")?;
    Ok((SocketAddr::new(external, port), Lease { gateway, port }))
}
async fn teacher_worker(s: Arc<Shared>, options: TeacherOptions) -> Result<(), String> {
    if !s.active.load(SeqCst) {
        return Ok(());
    }
    let ip = if options.loopback {
        Ipv4Addr::LOCALHOST
    } else {
        Ipv4Addr::UNSPECIFIED
    };
    let listener = TcpListener::bind((ip, 0))
        .await
        .map_err(|_| "Cannot listen for student")?;
    let local = listener
        .local_addr()
        .map_err(|_| "Cannot read listener address")?;
    // Discovery is isolated on this session's worker. The library never changes firewall rules.
    let (endpoint, _lease) = if options.loopback {
        (local, None)
    } else {
        let (e, l) = mapping(local.port(), &s.active)?;
        (e, Some(l))
    };
    if !s.active.load(SeqCst) {
        return Ok(());
    }
    let rcgen::CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(vec!["direct-session".into()])
            .map_err(|_| "Cannot create session certificate")?;
    let der = cert.der().clone();
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|_| "TLS configuration failed")?
    .with_no_client_auth()
    .with_single_cert(
        vec![der.clone()],
        PrivatePkcs8KeyDer::from(signing_key.serialize_der()).into(),
    )
    .map_err(|_| "TLS certificate failed")?;
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let mut token = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut token);
    let token = URL_SAFE_NO_PAD.encode(token);
    let invitation = Invitation {
        version: 3,
        endpoint,
        cert: URL_SAFE_NO_PAD.encode(der.as_ref()),
        token: token.clone(),
        expires: now() + INVITE_SECONDS,
    };
    s.event(Event::Invitation(format!(
        "YTR3.{}",
        URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&invitation).map_err(|_| "Invitation encoding failed")?)
    )));
    let deadline = Instant::now() + Duration::from_secs(INVITE_SECONDS);
    let mut attempts = 0;
    loop {
        if !s.active.load(SeqCst) || Instant::now() >= deadline {
            return Ok(());
        }
        let accepted = tokio::select! { _=s.wake.notified()=>{continue;}, r=timeout(Duration::from_millis(200),listener.accept())=>r};
        let Ok(Ok((tcp, _))) = accepted else {
            continue;
        };
        tcp.set_nodelay(true)
            .map_err(|_| "Cannot configure direct connection")?;
        attempts += 1;
        if attempts > 32 {
            return Err("Too many connection attempts".into());
        }
        let tls = tokio::select! {_=s.wake.notified()=>{continue;},r=timeout(HANDSHAKE,acceptor.accept(tcp))=>r};
        let Ok(Ok(tls)) = tls else {
            continue;
        };
        let expected = format!("Bearer {token}");
        let ws = timeout(
            HANDSHAKE,
            tokio_tungstenite::accept_hdr_async_with_config(
                tls,
                Authorization { expected },
                Some(ws_config()),
            ),
        )
        .await;
        if let Ok(Ok(ws)) = ws {
            if Instant::now() >= deadline || !s.active.load(SeqCst) {
                return Ok(());
            }
            drop(listener);
            s.event(Event::Connected);
            return run_socket(s, Role::Teacher, ws).await;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
struct Authorization {
    expected: String,
}
impl tokio_tungstenite::tungstenite::handshake::server::Callback for Authorization {
    fn on_request(
        self,
        req: &Request,
        response: Response,
    ) -> std::result::Result<
        Response,
        tokio_tungstenite::tungstenite::handshake::server::ErrorResponse,
    > {
        let actual = req
            .headers()
            .get(http::header::AUTHORIZATION)
            .map(|x| x.as_bytes())
            .unwrap_or_default();
        if req.uri().path() != "/session"
            || req.headers().contains_key(http::header::ORIGIN)
            || !bool::from(actual.ct_eq(self.expected.as_bytes()))
        {
            let mut denied = http::Response::new(Some("Unauthorized".to_string()));
            *denied.status_mut() = StatusCode::UNAUTHORIZED;
            return Err(denied);
        }
        Ok(response)
    }
}
fn ws_config() -> WebSocketConfig {
    WebSocketConfig {
        max_message_size: Some(MAX_MESSAGE),
        max_frame_size: Some(MAX_MESSAGE),
        max_write_buffer_size: MAX_MESSAGE + 65536,
        ..Default::default()
    }
}
async fn student_worker(s: Arc<Shared>, i: Invitation) -> Result<(), String> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from(
            URL_SAFE_NO_PAD
                .decode(i.cert)
                .map_err(|_| "Invalid certificate")?,
        ))
        .map_err(|_| "Invalid certificate")?;
    let config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .map_err(|_| "TLS configuration failed")?
    .with_root_certificates(roots)
    .with_no_client_auth();
    let connect = async {
        let tcp = TcpStream::connect(i.endpoint)
            .await
            .map_err(|_| "Direct connection failed; network may require port mapping")?;
        tcp.set_nodelay(true)
            .map_err(|_| "Cannot configure direct connection")?;
        let tls = tokio_rustls::TlsConnector::from(Arc::new(config))
            .connect(
                ServerName::try_from("direct-session").unwrap().to_owned(),
                tcp,
            )
            .await
            .map_err(|_| "Pinned TLS certificate validation failed")?;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let mut req = format!("wss://{}/session", i.endpoint)
            .into_client_request()
            .map_err(|_| "Invalid endpoint")?;
        req.headers_mut().insert(
            http::header::AUTHORIZATION,
            format!("Bearer {}", i.token)
                .parse()
                .map_err(|_| "Invalid authorization")?,
        );
        tokio_tungstenite::client_async_with_config(req, tls, Some(ws_config()))
            .await
            .map(|(ws, _)| ws)
            .map_err(|_| "Invitation authentication failed")
    };
    let ws = tokio::select! {_=s.wake.notified()=>{return Ok(());},r=timeout(HANDSHAKE,connect)=>r.map_err(|_|"Connection timed out")??};
    if !s.active.load(SeqCst) {
        return Ok(());
    }
    s.event(Event::Connected);
    run_socket(s, Role::Student, ws).await
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", deny_unknown_fields)]
enum Wire {
    Approved,
    Input(InputCommand),
    Ack(u64),
}
fn encode_frame(f: Frame) -> Vec<u8> {
    let mut b = Vec::with_capacity(21 + f.jpeg.len());
    b.push(1);
    b.extend(f.sequence.to_be_bytes());
    b.extend(f.width.to_be_bytes());
    b.extend(f.height.to_be_bytes());
    b.extend([0u8; 4]);
    b.extend(f.jpeg);
    b
}
fn decode_frame(b: &[u8]) -> Option<Frame> {
    if b.len() < 21 || b[0] != 1 {
        return None;
    }
    let f = Frame {
        sequence: u64::from_be_bytes(b[1..9].try_into().ok()?),
        width: u32::from_be_bytes(b[9..13].try_into().ok()?),
        height: u32::from_be_bytes(b[13..17].try_into().ok()?),
        jpeg: b[21..].to_vec(),
    };
    f.valid().then_some(f)
}
async fn run_socket<S>(s: Arc<Shared>, role: Role, mut ws: WebSocketStream<S>) -> Result<(), String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let start = Instant::now();
    let mut last_rx = start;
    let mut tick = tokio::time::interval(Duration::from_millis(5));
    let mut window = Instant::now();
    let mut messages = 0;
    let mut last_ping = Instant::now();
    let mut pending_ping = None;
    let mut in_flight: VecDeque<(u64, Instant)> = VecDeque::new();
    loop {
        if in_flight
            .front()
            .is_some_and(|(_, sent)| sent.elapsed() > Duration::from_secs(2))
        {
            return Err("The viewer stopped acknowledging frames".into());
        }
        if !s.active.load(SeqCst)
            || start.elapsed() > LIFETIME
            || last_rx.elapsed() > IDLE
            || (!s.approved.load(SeqCst) && start.elapsed() > Duration::from_secs(120))
        {
            s.revoke();
            let _ = timeout(Duration::from_millis(100), ws.close(None)).await;
            return Ok(());
        }
        tokio::select! {
            _=s.wake.notified()=>{},
            _=tick.tick()=>{},
            incoming=ws.next()=> {
                let Some(Ok(message))=incoming else {return Ok(());};last_rx=Instant::now();
                if window.elapsed()>=Duration::from_secs(1){window=Instant::now();messages=0;}messages+=1;if messages>512{return Err("Peer message rate exceeded".into());}
                s.stats.received.fetch_add(1,SeqCst);
                match message {
                    Message::Close(_)=>return Ok(()),
                    Message::Ping(_)=>{},
                    Message::Pong(data)=>{if let Some((nonce,at))=pending_ping {if data==nonce {s.stats.last_rtt_ms.store(Instant::now().duration_since(at).as_millis() as u64,SeqCst);pending_ping=None;}}},
                    Message::Binary(data)=>{
                        let frame=decode_frame(&data).ok_or("Invalid frame")?;let mut b=s.boxes.lock().unwrap();
                        if role!=Role::Teacher||!s.active.load(SeqCst)||!s.approved.load(SeqCst){return Err("Unauthorized frame".into());}
                        if b.incoming.replace(frame).is_some(){s.stats.dropped.fetch_add(1,SeqCst);}
                    },
                    Message::Text(text)=>{
                        if text.len()>2048{return Err("Control message too large".into());}
                        let wire:Wire=serde_json::from_str(&text).map_err(|_|"Invalid control message")?;
                        let mut b=s.boxes.lock().unwrap();
                        match wire {
                            Wire::Approved if role==Role::Teacher&&!s.approved.load(SeqCst)=>{s.approved.store(true,SeqCst);drop(b);s.event(Event::Approved);},
                            Wire::Input(input) if role==Role::Student&&s.active.load(SeqCst)&&s.approved.load(SeqCst)&&input.valid()=>{if b.inputs.len()>=INPUT_CAPACITY{return Err("Input queue overloaded".into());}b.inputs.push_back(input);},
                            Wire::Ack(sequence) if role==Role::Student&&s.approved.load(SeqCst)=>{
                                if let Some((_,sent))=in_flight.iter().find(|(seq,_)|*seq==sequence) {
                                    s.stats.last_rtt_ms.store(sent.elapsed().as_millis().min(u64::MAX as u128) as u64,SeqCst);
                                    while in_flight.front().is_some_and(|(seq,_)|*seq<=sequence){in_flight.pop_front();}
                                }
                            },
                            _=>return Err("Unauthorized control message".into()),
                        }
                    },
                    _=>return Err("Unsupported message".into()),
                }
            }
        }
        // Extract one bounded command; every iteration observes local revocation before writing.
        let outbound = {
            let mut b = s.boxes.lock().unwrap();
            if !s.active.load(SeqCst) {
                None
            } else if let Some(c) = b.commands.pop_front() {
                Some(Message::Text(
                    serde_json::to_string(&c).map_err(|_| "Control encoding failed")?,
                ))
            } else if s.approved.load(SeqCst) && in_flight.len() < 3 {
                b.outgoing.take().map(|f| {
                    in_flight.push_back((f.sequence, Instant::now()));
                    Message::Binary(encode_frame(f))
                })
            } else {
                None
            }
        };
        if let Some(message) = outbound {
            tokio::select! {_=cancelled(&s)=>{return Ok(());},r=timeout(Duration::from_secs(2),ws.send(message))=>{r.map_err(|_|"Peer write timeout")?.map_err(|_|"Peer disconnected")?;s.stats.sent.fetch_add(1,SeqCst);}}
        }
        if last_ping.elapsed() > Duration::from_secs(5) {
            let nonce = (start.elapsed().as_millis() as u64).to_be_bytes();
            timeout(
                Duration::from_secs(2),
                ws.send(Message::Ping(nonce.to_vec())),
            )
            .await
            .map_err(|_| "Ping timeout")?
            .map_err(|_| "Peer disconnected")?;
            pending_ping = Some((nonce, Instant::now()));
            last_ping = Instant::now();
        }
    }
}

async fn cancelled(shared: &Shared) {
    while shared.active.load(SeqCst) {
        shared.wake.notified().await;
    }
}

#[cfg(test)]
mod bounds_tests {
    use super::*;
    #[test]
    fn overflowing_input_queue_revokes_instead_of_losing_a_release() {
        let session = Session::new(Role::Teacher);
        session.approved.store(true, SeqCst);
        for _ in 0..64 {
            assert!(session.send_input(InputCommand::Key {
                name: "control".into(),
                pressed: true
            }));
        }
        assert!(!session.send_input(InputCommand::Key {
            name: "control".into(),
            pressed: false
        }));
        assert!(!session.active.load(SeqCst));
        assert!(!session.approved.load(SeqCst));
        assert!(session.shared.boxes.lock().unwrap().commands.is_empty());
    }
    #[test]
    fn slow_transport_keeps_only_one_pending_latest_frame_and_stop_discards_it() {
        let session = Session::new(Role::Student);
        assert!(session.allow());
        for sequence in 1..=100 {
            assert!(session.publish_frame(Frame {
                sequence,
                width: 1,
                height: 1,
                jpeg: vec![1]
            }));
        }
        assert_eq!(
            session
                .shared
                .boxes
                .lock()
                .unwrap()
                .outgoing
                .as_ref()
                .unwrap()
                .sequence,
            100
        );
        assert_eq!(session.stats.dropped.load(SeqCst), 99);
        session.stop();
        assert!(session.shared.boxes.lock().unwrap().outgoing.is_none());
    }
}
