FROM rust:1-bookworm AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p yt-signaling-server

FROM debian:bookworm-slim
RUN useradd --system --no-create-home --uid 10001 ytremote
COPY --from=build /src/target/release/yt-signaling-server /usr/local/bin/yt-signaling-server
USER ytremote
ENV YT_REMOTE_BIND=0.0.0.0:8080
EXPOSE 8080
CMD ["yt-signaling-server"]
