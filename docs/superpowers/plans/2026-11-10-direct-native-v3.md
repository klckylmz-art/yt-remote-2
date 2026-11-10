# Direct native v3 implementation plan

> **For agentic workers:** Use superpowers:executing-plans for integration; independent transport, video and platform files may be delegated under dispatching-parallel-agents. Checkboxes record verified completion only.

**Goal:** Implement the approved teacher-hosted direct invitation flow with explicit student consent and responsive Windows/macOS clients.

**Architecture:** A small direct-core library owns pinned WSS invitations, bounded transport and leased discovery. The native desktop adapter owns screen/input operations behind local consent. Latest-frame mailboxes and an adaptive policy avoid stale frame queues.

**Tech stack:** Rust, Tokio, rustls/ring, rcgen, tungstenite, igd-next; existing XCap/Enigo/Minifb prototype; audited RustDesk RTT estimator under AGPL.

**Spec:** `docs/superpowers/specs/2026-11-10-direct-native-v3.md`.

## Global constraints
- Native Windows x64, macOS Intel and Apple Silicon; no web/mobile UI.
- Only screen, keyboard and mouse; explicit student approval before capture/input.
- Ten-minute single-use invitations, authenticated WSS; no TLS bypass.
- No public listener, router mapping, firewall modification or desktop capture in automated development tests.
- Do not claim hardware encoding, completed RustDesk engine, native installation or measured 60 FPS.
- Preserve checks and dependency lockfile; one coherent development PR, no stable release.

## Review focus
- Stop racing queued frame/input delivery must revoke synchronously and discard stale work.
- Slow or malicious peers must not block UI or trigger unlimited allocations/tasks.
- Long pasted invitations and cancellation during discovery/handshake must recover.
- Portrait/Retina/letterboxed frames and local overlays must map coordinates correctly.
- Certificate tampering, old invitations and disconnected sessions must fail closed.

### Task 1: Direct invitation and transport
Files: create `crates/direct-core/{Cargo.toml,src/*.rs,tests/*.rs}`; root integrates workspace/dependency lock.
Interfaces: `start_teacher(TeacherOptions) -> Session`, `start_student(&str) -> Result<Session,String>`;
`TeacherOptions::default()` discovers/leases a mapping; `TeacherOptions::loopback()` never touches a router.
`Session` has public `active, approved: Arc<AtomicBool>`, `stats: Arc<Stats>` and methods
`try_event()->Option<Event>`, `take_frame()->Option<Frame>`, `try_input()->Option<InputCommand>`,
`allow()`, `deny()`, `stop()`, `send_input(InputCommand)->bool`, `publish_frame(Frame)->bool`, `ack_frame(u64)`.
`Event`: `Invitation(String)`, `Connected`, `Approved`, `Ended(String)`, `Error(String)`.
`Frame {sequence:u64,width:u32,height:u32,jpeg:Vec<u8>}`. `InputCommand` tagged JSON move/button/scroll/key,
coordinates normalized, keys include letters/digits and named modifiers including Mac command.
`Stats.last_rtt_ms: AtomicU64` plus sent/received/dropped counters. `stop/deny/drop` clear consent immediately.
- [ ] Write failing loopback tests for tampered/expired codes and certs, no frames/input before approval, deny/stop/disconnect and replay rejection, oversized messages and queue saturation.
- [ ] Run tests and observe the relevant failures; implement bounded TLS/WSS/discovery without production network exposure.
- [ ] Run crate tests and strict Clippy, documenting unsupported topology/cleanup limitations.

### Task 2: Video policy and RustDesk provenance
Files: create `crates/desktop/src/video.rs`, `third_party/rustdesk/rtt.rs`, `third_party/rustdesk/NOTICE.md`, root `COPYING` and `THIRD_PARTY_NOTICES.md`.
Interfaces: `video::Latest<T>::default(), put(T), take()->Option<T>, clear()`;
`AdaptiveVideo::default(), observe(processing:Duration, rtt:Duration), fps()->u32, interval()->Duration, quality()->u8`;
`Viewport::fit(remote_w:u32,remote_h:u32,view_w:usize,view_h:usize)` and `remote_point(x:f32,y:f32)->Option<(f64,f64)>`.
Viewport exposes integer `x,y,width,height` for rendering. Invalid dimensions return `None` from fit.
- [ ] Write failing tests for latest-frame replacement, 60→30 adaptation/recovery hysteresis, growing network queue pressure, and portrait/letterbox coordinate boundaries.
- [ ] Reuse and attribute upstream RustDesk RttCalculator from revision `95cca3f661f635b3cc85c04935fe8d96d8f432ef`; retain AGPL text and document modifications.
- [ ] Implement policy and run tests. This is initial QoS reuse, not completed capture/codec integration or proof of FPS.

### Task 3: Native platform and packaging
Files: create `crates/desktop/src/platform.rs`; modify `packaging/macos/package.sh`; add appropriate docs/tests.
Interfaces: `ensure_capture_permissions()->Result<(),String>` called ONLY after explicit student allow;
`minimize(&minifb::Window)->Result<(),String>`; `PRODUCT_NAME: &str = "YT Remote 3.0"`.
- [ ] Review Mac Screen Recording/Accessibility behavior and Minifb native handles; implement native minimize and permission preflight without bypassing OS grants.
- [ ] Update bundle identity/version/display name and relevant permission description/notices while keeping ad-hoc signature and architecture validation.
- [ ] Validate scripts/manifest behavior with available tools; actual Mac grants/install remain physical-test blockers.

### Task 4: Native integration and verification
Files: create `crates/desktop/src/direct_gui.rs`; modify main/Cargo/UI/video adapters and README; keep old relay mode developer-only.
- [ ] Test invitation paste preservation and viewport/control translation before implementing their adapters.
- [ ] Implement responsive teacher/student pages, worker-owned capture/decode, scoped input state and cleanup, bounded latest-frame rendering, local minimize/disconnect and long-code paste/copy.
- [ ] Enforce image decode limits; native screen permissions only after approval; encode at original aspect within 1080p with explicit JPEG quality and adaptive 60/30 target, no static-screen retransmission.
- [ ] Run fmt, workspace check/test/strict Clippy and native Windows release build; run physical-independent loopback WSS tests.
- [ ] Independent whole-change review, fix actual defects, then publish one PR and inspect Windows/macOS CI artifacts. Document every unverified release gate and remaining RustDesk engine work.
