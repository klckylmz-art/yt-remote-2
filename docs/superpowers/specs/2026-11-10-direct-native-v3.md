# YT Remote 3.0 direct native development specification

The user approved a teacher-generated invitation pasted by the student.
The teacher controls the STUDENT PC; who initiates the network connection
does not determine control direction. Support native Windows x64 and macOS
Intel/Apple Silicon. Interpret the latest 'amc' request as Mac support.

The teacher accepts one student-initiated connection. Invitations carry an
endpoint, ephemeral TLS certificate and random bearer capability, expire
after ten minutes, and are consumed by one authenticated connection.
Student explicitly allows SCREEN + KEYBOARD + MOUSE before capture, frames
or injected input. DENY, CANCEL, STOP, socket loss and expiration revoke
access and release held input. No accounts, passwords, audio, camera, file
transfer, clipboard synchronization or unattended approval.

Use authenticated WSS with invitation-pinned certificate trust through the
standard TLS verifier; never accept all certificates or bypass TLS checks.
Bound messages, queues, handshake duration, idle duration and session life.
No public listener, router mapping, firewall rule or screen capture is to
be started during development tests. Test on loopback without capturing a
desktop. In the actual app, generating an invitation may request a leased
UPnP mapping, removed on cancellation, failure, expiry and exit. No permanent
mapping fallback. Do not alter OS firewall automatically.

The direct self-contained invitation is longer than the former 16-character
server lookup code. It must be copied/pasted intact; never display private
keys, tokens, raw endpoint configuration or require IP entry from users.
Keep old relay prototype as an explicit developer fallback, not the new UI.

Keep UI responsive: worker threads own discovery, TLS, WebSocket I/O,
capture and decode; the UI only polls bounded events and latest frames.
Video preserves aspect ratio, targets up to 1920x1080 / 60 FPS and adapts
to 30 FPS under sustained processing/ack pressure. Never claim achieved
FPS or improvement over RustDesk without measured equivalent benchmarks.
Reuse/audit RustDesk components rather than representing a new JPEG client
as a completed RustDesk engine. Initially reuse its bounded RTT-baseline
estimator from an immutable revision, with AGPL notices. Full native
capture/hardware-codec integration remains a separate explicit blocker.

English-only UI: teacher GENERATE CODE (TEACHER), COPY CODE and CANCEL;
student paste code + CONNECT, ALLOW SCREEN & KEYBOARD, DENY, STOP SHARING NOW.
Teacher viewer includes local minimize/disconnect, letterboxing and correct
remote coordinate mapping. OS screen/accessibility grants stay user-controlled.
Latest user steering: different cities and different modems; all mouse,
keyboard and application work occurs on the STUDENT PC. NoMachine is a
quality benchmark target, not a claimed equivalence. On a student Mac,
request/check Screen Recording, Accessibility and then Input Monitoring,
in order, only following explicit session approval. No global key recorder
or event listener is to be added merely because that third grant was requested.

No stable release: native CI artifacts remain UNTESTED until clean-machine
install, OS grant, real separate-network and performance tests pass.
