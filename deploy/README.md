# Signaling + relay deployment template

This folder packages a Rust authorization/WebSocket relay behind Caddy HTTPS.
It is **not** deployed or production-audited. A real public host, reachable DNS,
firewall configuration, abuse prevention, persistent session cleanup, operational
monitoring, integration tests and end-to-end encryption must be addressed before
public release.

Only administrators configure the domain; students and teachers receive a
single session code and never configure IP/port/HTTPS URLs.

## Development deployment

1. Use an owned domain, create A/AAAA DNS records for the server and open TCP 80/443.
2. Copy `.env.example` to `.env` and set `YT_REMOTE_DOMAIN`.
3. In `deploy/`, run `docker compose up -d --build`.
4. Verify HTTPS and `GET https://your.domain/health`.
5. Point the development desktop clients at `YT_REMOTE_SERVER=https://your.domain`.

The backend is confined to the internal Docker network and Caddy is the only
public listener. Caddy handles the WebSocket upgrade automatically. Never expose
the backend port 8080 directly to the public internet.

This is a custom HTTPS WebSocket relay, not a standard WebRTC TURN server.
Its existence in source code is **not** proof that a relay has actually been
deployed or tested under restrictive NAT.
