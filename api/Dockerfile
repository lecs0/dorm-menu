# Build on a stronger machine. For ARMv7, use buildx + binfmt/QEMU; no NAS compilation.
FROM rust:1.99.1-bookworm AS builder
WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY shared shared
COPY api api
COPY app/Cargo.toml app/Cargo.toml
COPY app/src app/src
RUN cargo build --locked --release -p dorm-api
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates poppler-utils && rm -rf /var/lib/apt/lists/* \
    && groupadd -g 10001 dorm && useradd -u 10001 -g dorm -M dorm && mkdir /data && chown dorm:dorm /data
COPY --from=builder /build/target/release/dorm-api /usr/local/bin/dorm-api
USER 10001:10001
ENV BIND=0.0.0.0:3000 DATA_DIR=/data TOKEN_FILE=/config/tokens.json RUST_LOG=info
VOLUME /data
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/dorm-api"]
