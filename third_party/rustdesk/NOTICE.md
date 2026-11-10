# RustDesk component provenance

`rtt.rs` is adapted from `RttCalculator` in RustDesk's
`src/server/video_qos.rs`, revision
`95cca3f661f635b3cc85c04935fe8d96d8f432ef`.

Upstream: https://github.com/rustdesk/rustdesk/blob/95cca3f661f635b3cc85c04935fe8d96d8f432ef/src/server/video_qos.rs

Copyright: RustDesk contributors. License: GNU Affero General Public License
version 3; see the complete license in the repository root `COPYING`.
Modification: isolate the RTT baseline estimator, add explicit std imports
and crate visibility. The native adapter's FPS policy is separate code.

This is the first reused QoS component. RustDesk capture, codec and complete
transport engine integration is NOT complete. No hardware acceleration or
performance superiority is established by this reuse.
