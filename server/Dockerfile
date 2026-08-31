# Build the webhook binary, then ship it on a slim runtime.
# sqlite is compiled into the binary (libsqlite3-sys "bundled"), so the runtime
# image needs no sqlite package.
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --bin indeed-key-webhook

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 app \
    && mkdir -p /data \
    && chown app /data
COPY --from=builder /app/target/release/indeed-key-webhook /usr/local/bin/indeed-key-webhook
USER app
# WEBHOOK_SECRET has no default and must be supplied at run time.
ENV BIND_ADDR=0.0.0.0:8080 \
    DB_PATH=/data/codes.db
EXPOSE 8080
VOLUME ["/data"]
ENTRYPOINT ["indeed-key-webhook"]
