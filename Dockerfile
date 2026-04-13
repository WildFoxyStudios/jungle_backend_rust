# ─── Stage 1: Build ──────────────────────────────────────────────────────────
FROM rust:bookworm AS builder

WORKDIR /app

# Install system deps for sqlx/openssl
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY crates/shared/Cargo.toml crates/shared/Cargo.toml
COPY crates/auth-service/Cargo.toml crates/auth-service/Cargo.toml
COPY crates/user-service/Cargo.toml crates/user-service/Cargo.toml
COPY crates/post-service/Cargo.toml crates/post-service/Cargo.toml
COPY crates/messaging-service/Cargo.toml crates/messaging-service/Cargo.toml
COPY crates/media-service/Cargo.toml crates/media-service/Cargo.toml
COPY crates/notification-service/Cargo.toml crates/notification-service/Cargo.toml
COPY crates/group-page-service/Cargo.toml crates/group-page-service/Cargo.toml
COPY crates/content-service/Cargo.toml crates/content-service/Cargo.toml
COPY crates/commerce-service/Cargo.toml crates/commerce-service/Cargo.toml
COPY crates/admin-service/Cargo.toml crates/admin-service/Cargo.toml
COPY crates/payment-service/Cargo.toml crates/payment-service/Cargo.toml
COPY crates/realtime-service/Cargo.toml crates/realtime-service/Cargo.toml
COPY crates/api-gateway/Cargo.toml crates/api-gateway/Cargo.toml
COPY crates/jobs-runner/Cargo.toml crates/jobs-runner/Cargo.toml
COPY crates/ai-service/Cargo.toml crates/ai-service/Cargo.toml

# Create empty lib.rs / main.rs stubs for dependency caching
RUN mkdir -p crates/shared/src && echo "pub fn _stub() {}" > crates/shared/src/lib.rs
RUN for d in auth-service user-service post-service messaging-service media-service notification-service group-page-service content-service commerce-service admin-service payment-service realtime-service api-gateway jobs-runner ai-service; do \
      mkdir -p crates/$d/src && echo "fn main() {}" > crates/$d/src/main.rs; \
    done

# Cache dependency build
ENV SQLX_OFFLINE=true
RUN cargo build --release --workspace

# Copy real source
COPY crates/ crates/
COPY migrations/ migrations/

# Touch all main files to invalidate cache for real sources
RUN find crates -name "*.rs" -exec touch {} +

# Build all binaries
RUN cargo build --release --workspace

# ─── Stage 2: Runtime ────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates libssl3 curl tini && \
    rm -rf /var/lib/apt/lists/*

RUN groupadd --system app && useradd --system --gid app --create-home app

# Copy all binaries
COPY --from=builder /app/target/release/auth-service /usr/local/bin/
COPY --from=builder /app/target/release/user-service /usr/local/bin/
COPY --from=builder /app/target/release/post-service /usr/local/bin/
COPY --from=builder /app/target/release/messaging-service /usr/local/bin/
COPY --from=builder /app/target/release/media-service /usr/local/bin/
COPY --from=builder /app/target/release/notification-service /usr/local/bin/
COPY --from=builder /app/target/release/group-page-service /usr/local/bin/
COPY --from=builder /app/target/release/content-service /usr/local/bin/
COPY --from=builder /app/target/release/commerce-service /usr/local/bin/
COPY --from=builder /app/target/release/admin-service /usr/local/bin/
COPY --from=builder /app/target/release/payment-service /usr/local/bin/
COPY --from=builder /app/target/release/realtime-service /usr/local/bin/
COPY --from=builder /app/target/release/api-gateway /usr/local/bin/
COPY --from=builder /app/target/release/jobs-runner /usr/local/bin/
COPY --from=builder /app/target/release/ai-service /usr/local/bin/

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app
USER app

ENTRYPOINT ["tini", "--"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:${SERVER_PORT:-8080}/health || exit 1
