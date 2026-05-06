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
COPY crates/live-service/Cargo.toml crates/live-service/Cargo.toml
COPY crates/api-gateway/Cargo.toml crates/api-gateway/Cargo.toml
COPY crates/jobs-runner/Cargo.toml crates/jobs-runner/Cargo.toml
COPY crates/ai-service/Cargo.toml crates/ai-service/Cargo.toml

# Create empty lib.rs / main.rs stubs for dependency caching
RUN mkdir -p crates/shared/src && echo "pub fn _stub() {}" > crates/shared/src/lib.rs
RUN for d in auth-service user-service post-service messaging-service media-service notification-service group-page-service content-service commerce-service admin-service payment-service realtime-service live-service api-gateway jobs-runner ai-service; do \
      mkdir -p crates/$d/src && echo "fn main() {}" > crates/$d/src/main.rs; \
    done

# Cache dependency build
ENV SQLX_OFFLINE=true
ENV CARGO_BUILD_JOBS=1
ENV CARGO_PROFILE_RELEASE_OPT_LEVEL=2
# Limit rustc codegen units to reduce peak memory per compilation unit
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
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
COPY --from=builder /app/target/release/live-service /usr/local/bin/
COPY --from=builder /app/target/release/api-gateway /usr/local/bin/
COPY --from=builder /app/target/release/jobs-runner /usr/local/bin/
COPY --from=builder /app/target/release/ai-service /usr/local/bin/

# Copy migrations
COPY --from=builder /app/migrations /app/migrations

WORKDIR /app
RUN mkdir -p /app/uploads && chown -R app:app /app/uploads

# ── Startup script: runs all 15 services + jobs-runner ──
RUN printf '#!/bin/bash\n\
set -e\n\
\n\
PIDS=""\n\
\n\
start_svc() {\n\
  local name=$1\n\
  local port=$2\n\
  echo "Starting ${name} on port ${port}..."\n\
  /usr/local/bin/${name} &\n\
  PIDS="$PIDS $!"\n\
}\n\
\n\
# API Gateway (main entry point)\n\
start_svc api-gateway "${SERVER_PORT:-8080}"\n\
\n\
# Internal services\n\
start_svc auth-service         3001\n\
start_svc user-service         3002\n\
start_svc post-service         3003\n\
start_svc messaging-service    3004\n\
start_svc media-service        3005\n\
start_svc notification-service 3006\n\
start_svc group-page-service   3007\n\
start_svc content-service      3008\n\
start_svc commerce-service     3009\n\
start_svc admin-service        3010\n\
start_svc payment-service      3011\n\
start_svc realtime-service     3012\n\
start_svc ai-service           3013\n\
start_svc live-service         3014\n\
\n\
# Background worker\n\
echo "Starting jobs-runner..."\n\
/usr/local/bin/jobs-runner &\n\
PIDS="$PIDS $!"\n\
\n\
trap "kill $PIDS 2>/dev/null; exit 0" SIGTERM SIGINT\n\
wait -n\n\
' > /usr/local/bin/start.sh && chmod +x /usr/local/bin/start.sh

USER app

ENTRYPOINT ["tini", "--"]
CMD ["/usr/local/bin/start.sh"]

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -f http://localhost:${SERVER_PORT:-8080}/health || exit 1
