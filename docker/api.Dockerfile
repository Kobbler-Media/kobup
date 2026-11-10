FROM rust:1.99-trixie AS base
WORKDIR /app
RUN cargo install cargo-watch
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY .sqlx ./.sqlx

FROM base AS dev
ENV SQLX_OFFLINE=true
ENV CARGO_TARGET_DIR=/target
RUN cargo fetch
CMD ["cargo", "watch", "-x", "run -p kobup-service"]

FROM base AS builder
ENV SQLX_OFFLINE=true
# Cache mounts outlive `make docker-push`'s --no-cache. The target mount is gone once this RUN
# ends, so the binary is copied out.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,id=kobup-target,target=/app/target \
    cargo build --release -p kobup-service && \
    cp /app/target/release/kobup-service /app/kobup-service-binary

FROM debian:trixie-slim AS prod
LABEL org.opencontainers.image.source="https://github.com/Kobbler-Media/kobup"
RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/kobup-service-binary /app/kobup-service
ENV BIND_ADDR=0.0.0.0:8080
EXPOSE 8080
CMD ["/app/kobup-service"]
