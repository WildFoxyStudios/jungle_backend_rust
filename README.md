# Jungle Backend — Rust Microservices

Reescritura completa del backend de la red social Jungle (PHP) en **Rust**, con arquitectura de microservicios lista para producción.

**519 endpoints · 15 servicios · 20 gateways de pago · 3 proveedores IA · 14 OAuth social**

## 📚 Documentación Completa

→ **[Ver documentación completa](./docs/README.md)**  
→ **[Resumen ejecutivo del proyecto](./docs/OVERVIEW.md)**

| Documento | Descripción |
|-----------|-------------|
| [Resumen Ejecutivo](./docs/OVERVIEW.md) | Qué incluye, números clave, todas las funcionalidades |
| [Arquitectura](./docs/architecture.md) | Diagrama de servicios, stack tecnológico, flujo de requests |
| [API Gateway](./docs/api-gateway.md) | Routing completo, rate limiting, WebSocket proxy |
| [Configuración](./docs/configuration.md) | Variables de entorno, Docker, desarrollo local |
| [Autenticación](./docs/auth.md) | JWT, 2FA TOTP, OAuth social (14 proveedores) |
| [API Reference](./docs/api/) | Documentación de cada servicio (13 documentos) |
| [Modelos de Datos](./docs/data-models.md) | Estructuras reales con ejemplos JSON |
| [Internals](./docs/internals.md) | Algoritmos, transacciones, detalles de implementación |
| [Event Bus](./docs/event-bus.md) | 28 eventos NATS, DLQ |
| [Background Jobs](./docs/jobs.md) | 20 tareas en segundo plano |
| [Payment Gateways](./docs/payment-gateways.md) | 20 proveedores de pago |
| [Base de Datos](./docs/database.md) | 28 migraciones SQL, esquema |
| [Observabilidad](./docs/observability.md) | Prometheus, OpenTelemetry |
| [Resiliencia](./docs/resilience.md) | Circuit breaker, retry, DLQ |
| [Migración MySQL→PG](./docs/migration.md) | Herramienta de migración desde PHP |
| [Matriz de Entorno](./docs/env-matrix.md) | Variables obligatorias por entorno y sensibilidad |
| [Producción en Fly.io](./docs/production-flyio.md) | Guía de despliegue backend en Fly |
| [Runbook de Despliegue](./docs/runbooks/deploy.md) | Procedimiento operativo con rollback |

> Swagger UI interactivo: `http://localhost:8080/swagger-ui`

## Architecture

```
Client → API Gateway (:8080) → Microservices (:3001-3013)
                                     ↓
                        PostgreSQL 16 + Redis 7 + NATS + MinIO
```

### Services

| Service | Port | Description |
|---|---|---|
| **api-gateway** | 8080 | Reverse proxy, rate limiting, routing |
| **auth-service** | 3001 | Registration, login, JWT, sessions, 2FA |
| **user-service** | 3002 | Profiles, search, follow/block/poke/mute |
| **post-service** | 3003 | Feed, posts, reactions, comments, polls, hashtags |
| **messaging-service** | 3004 | Conversations, messages, typing, broadcasts |
| **media-service** | 3005 | Uploads, avatars, covers, stories |
| **notification-service** | 3006 | Notifications CRUD, preferences |
| **group-page-service** | 3007 | Pages, groups, events |
| **content-service** | 3008 | Blogs, forums, movies, games |
| **commerce-service** | 3009 | Products, orders, jobs, funding, offers |
| **admin-service** | 3010 | Dashboard, user mgmt, reports, config |
| **payment-service** | 3011 | 8 real gateways + 12 stubs, wallet, pro/creator subs |
| **realtime-service** | 3012 | WebSocket, presence, typing, WebRTC signaling |
| **ai-service** | 3013 | OpenAI chat, post suggestions, image description |
| **jobs-runner** | — | 11 background tasks (cron replacement) |

### Tech Stack

- **Rust** 1.86+, edition 2024
- **Axum** 0.8 (HTTP framework)
- **SQLx** 0.8 (PostgreSQL, runtime queries)
- **Redis** 7 (caching, rate limiting, pub/sub)
- **MinIO** (S3-compatible object storage)
- **JWT** authentication with refresh tokens
- **NATS** (event bus, async inter-service communication)
- **Prometheus** metrics on `/metrics` in every service
- Cursor-based pagination

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Rust 1.86+ (for local development)

### With Docker (recommended)

```bash
# Copy environment file
cp .env.example .env

# Start everything (infrastructure + all services)
docker compose up -d

# Check health
curl http://localhost:8080/health
```

### Local Development

```bash
# Start only infrastructure
docker compose up -d postgres redis nats minio

# Copy env file
cp .env.example .env

# Run a specific service
cargo run -p auth-service

# Run all checks
cargo check --workspace

# Run the jobs runner
cargo run -p jobs-runner
```

## SQL Migrations

Migrations run automatically on service startup via `sqlx::migrate!()`.

| Migration | Tables |
|---|---|
| 001_initial_schema | users, sessions, backup_codes, login_attempts, banned_ips |
| 002_social_graph | follows, blocks, pokes, mutes, family_relations, experience, skills |
| 003_posts_content | posts, reactions, comments, polls, saved/hidden posts, hashtags |
| 004_messaging | conversations, conversation_members, messages, broadcasts |
| 005_media_stories | stories, story_media, story_views, albums, uploaded_media |
| 006_notifications | notifications, notification_settings |
| 007_groups_pages_events | categories, pages, groups, events, responses |
| 008_content | blogs, blog_comments, forum_sections, forums, threads, replies, movies, games |
| 009_commerce | products, reviews, orders, jobs, applications, fundings, donations, offers |
| 010_payments_ads_config | payment_transactions, withdrawals, ads, site_config, translations, reports |
| 011_remaining | calls, pro_subscriptions, creator_tiers, stickers, gifts, oauth, etc. |

## Project Structure

```
backend/
├── Cargo.toml              # Workspace root
├── Dockerfile              # Multi-stage build
├── docker-compose.yml      # Full stack orchestration
├── .env.example
├── migrations/             # SQL migrations (PostgreSQL)
└── crates/
    ├── shared/             # Common: auth, config, errors, db, events, metrics, resilience, i18n, pagination
    ├── auth-service/
    ├── user-service/
    ├── post-service/
    ├── messaging-service/
    ├── media-service/
    ├── notification-service/
    ├── group-page-service/
    ├── content-service/
    ├── commerce-service/
    ├── admin-service/
    ├── payment-service/    # PaymentGateway trait + 8 real gateways + 12 stubs
    ├── realtime-service/   # WebSocket hub + presence + NATS event relay
    ├── api-gateway/        # Reverse proxy + rate limiting
    ├── jobs-runner/        # 11 background tasks
    └── ai-service/         # OpenAI integration
```

## OpenAPI / Swagger UI

Swagger UI is available at `http://localhost:8080/swagger-ui` when the API Gateway is running.
The OpenAPI JSON spec is served at `http://localhost:8080/api-docs/openapi.json`.

All 519 endpoints across 15 tag groups are documented.

## Tests

```bash
# Run all workspace tests
cargo test --workspace

# Run tests for a specific crate
cargo test -p shared
cargo test -p media-service
cargo test -p payment-service
```

Current test coverage: 34 unit tests across shared (23), media-service (6), payment-service (5).

## API Examples

```bash
# Register
curl -X POST http://localhost:8080/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"username":"john","email":"john@example.com","password":"secret123"}'

# Login
curl -X POST http://localhost:8080/v1/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"email":"john@example.com","password":"secret123"}'

# Get feed (with JWT)
curl http://localhost:8080/v1/feed \
  -H 'Authorization: Bearer <token>'

# WebSocket
wscat -c 'ws://localhost:8080/ws?token=<jwt>'
```

## Data Migration (MySQL → PostgreSQL)

```bash
pip install mysql-connector-python psycopg2-binary
python tools/migrate_mysql_to_pg.py \
  --mysql-host 127.0.0.1 --mysql-db Jungle_db \
  --pg-host 127.0.0.1 --pg-db Jungle
```

Migrates 60+ tables with type conversions, table consolidations, sequence resets, and verification.

## Rate Limits

| Endpoint | Limit |
|---|---|
| `/v1/auth/*` | 5 req / 15 min |
| `/v1/media/upload` | 20 req / min |
| `*/search*` | 30 req / min |
| `/v1/messages/*` | 60 req / min |
| Everything else | 100 req / min |
