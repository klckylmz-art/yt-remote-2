# NoMachine quality target — not an equivalence claim

The owner wants NoMachine-like responsiveness, Excel text clarity and smooth
mouse/keyboard control on the STUDENT PC, from another city/network.
The teacher is the controller regardless of the TCP initiation direction.

The current direct adapter is still a JPEG prototype. It reuses a pinned
RustDesk RTT estimator, but is NOT the completed RustDesk capture/codec engine.
Hardware H.264/H.265/AV1 integration is outstanding. NoMachine documents
hardware and software H.264; RustDesk documents software VP8/VP9/AV1 and
hardware H.264/H.265. Use audited upstream components rather than treating a
target FPS setting as evidence of equivalent performance.

Implemented development mechanisms to verify in real use:
- Pinned-certificate WSS directly between the two native apps.
- Dedicated student input worker, independent of capture/JPEG encoding.
- Latest-frame mailboxes and at most three unacknowledged frames; cumulative
  viewer acknowledgments permit dropped intermediate frames without deadlock.
- Adaptive 60/30/15/5 FPS target with hysteresis; JPEG quality remains 90 to
  prioritize formula text. The selected target is NOT measured delivered FPS.
- Aspect-preserving 1080p bounds, letterboxing and normalized mouse positions.
- Unchanged-screen frames are not repeatedly JPEG-encoded/transmitted.

Before any quality comparison, test on the same Windows/Mac pair, displays,
resolutions and network conditions, changing only the application. Record:
1. Capture-to-visible-display latency p50/p95, measured with a shared visual
   clock/camera method rather than subtracting unsynchronized host clocks.
2. Input-to-visible-response latency for mouse clicks, typing and Excel scroll.
3. Delivered frames, dropped/replaced frames, encoding/decoding time, CPU/GPU
   usage and bytes per second on both computers.
4. 1920x1080 Excel cell/formula readability, small fonts, rapid scroll and
   movement, including Retina/scaled displays and supported keyboard layouts.
5. Same-network vs real different-city modem paths, network loss/jitter,
   student STOP, disconnect/input release and fresh approval after reconnect.

Compare with both the unmodified pinned RustDesk version and NoMachine.
Retain test logs and hardware/OS/network details; do not claim gains without
them. Real native hardware-codec adoption and these measurements remain
release blockers. Direct connectivity is not guaranteed on every network.

Primary references:
- https://www.nomachine.com/support/faq
- https://kb.nomachine.com/AR04Q01022
- https://rustdesk.com/docs/en/
- https://github.com/rustdesk-org/hwcodec
