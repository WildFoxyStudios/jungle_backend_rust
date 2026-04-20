# Shared Crate

El crate `shared` (`crates/shared`) contiene todos los módulos comunes reutilizados por los 15 servicios. Es la base del sistema.

---

## Módulos

### `auth` — Autenticación JWT

Provee los extractores de Axum para autenticación:

- **`AuthUser`** — Extractor que requiere un Bearer token válido. Retorna `401` si falta o es inválido.
- **`OptionalAuth`** — Extractor que acepta requests con o sin token.
- **`Claims`** — Estructura del payload JWT (`sub`, `uuid`, `is_admin`, `exp`, `iat`).
- **`AppState`** — Estado compartido de la aplicación (DB pool, Redis, config, event bus).
- **`encode_access_token()`** — Genera un access token JWT con vida de 15 minutos.
- **`hash_token()`** — Hash SHA-256 de un token (para almacenar refresh tokens).

```rust
// Uso en un handler
async fn my_handler(auth: AuthUser, State(state): State<AppState>) -> impl IntoResponse {
    // auth.user_id: i64
    // auth.is_admin: bool
}
```

---

### `config` — Configuración

- **`AppConfig`** — Carga configuración desde variables de entorno via `dotenvy`.
- **`SharedConfig`** — Alias `Arc<AppConfig>` para compartir entre threads.

Campos: `database_url`, `redis_url`, `nats_url`, `jwt_secret`, `jwt_refresh_secret`, `server_host`, `server_port`, `frontend_url`, `allowed_origins`.

---

### `errors` — Manejo de Errores

- **`ApiError`** — Enum de errores de la API que implementa `IntoResponse` para Axum.

| Variante | HTTP Status | Código |
|----------|-------------|--------|
| `BadRequest(String)` | 400 | `BAD_REQUEST` |
| `Unauthorized` | 401 | `UNAUTHORIZED` |
| `Forbidden(String)` | 403 | `FORBIDDEN` |
| `NotFound(String)` | 404 | `NOT_FOUND` |
| `Conflict(String)` | 409 | `CONFLICT` |
| `Validation(Vec<FieldError>)` | 422 | `VALIDATION_ERROR` |
| `RateLimited` | 429 | `RATE_LIMITED` |
| `Internal(String)` | 500 | `INTERNAL_ERROR` |

Conversiones automáticas desde: `sqlx::Error`, `jsonwebtoken::Error`, `argon2::Error`, `redis::RedisError`, `validator::ValidationErrors`.

---

### `events` — Bus de Eventos

Ver [Event Bus](./event-bus.md) para documentación completa.

- **`DomainEvent`** — Enum con todos los eventos del dominio.
- **`EventBus`** trait — Abstracción sobre el bus de eventos.
- **`NatsEventBus`** — Implementación NATS con retry y DLQ.
- **`NoopEventBus`** — Implementación no-op para tests.

---

### `pagination` — Paginación

- **`PaginationParams`** — Query params `cursor` y `limit` (default: 20, max: 100).
- **`PaginatedResponse<T>`** — Respuesta paginada con `data` y `meta`.
- **`PaginationMeta`** — `cursor`, `has_more`, `total` (opcional).

```rust
// Uso en un handler
async fn list_items(
    Query(params): Query<PaginationParams>,
    State(state): State<AppState>,
) -> Result<Json<PaginatedResponse<Item>>, ApiError> {
    let limit = params.limit(); // 1-100, default 20
    let cursor_id = params.cursor_id(); // Option<i64>
    // ...
}
```

---

### `resilience` — Resiliencia

Ver [Resiliencia](./resilience.md) para documentación completa.

- **`CircuitBreaker`** — Abre tras N fallos consecutivos, se recupera tras timeout.
- **`retry_with_backoff()`** — Retry con backoff exponencial (100ms base, duplica cada intento).
- **`publish_with_retry()`** — Publicar en NATS con retry y DLQ fallback.
- **`retry_boxed()`** — Variante con futures boxeados.

---

### `metrics` — Métricas Prometheus

- **`metrics_handler()`** — Handler GET `/metrics` para scraping de Prometheus.
- **`metrics_middleware()`** — Middleware Axum que registra conteo y duración de requests.
- **`HTTP_REQUESTS`** — Counter: `http_requests_total{method, path, status}`.
- **`HTTP_DURATION`** — Histogram: `http_request_duration_seconds{method, path}`.
- **`DB_QUERIES`** — Counter: `db_queries_total{query_type}`.
- **`ACTIVE_WEBSOCKETS`** — Gauge: `Jungle_active_websocket_connections`.

Los IDs en paths se normalizan a `{id}` para evitar alta cardinalidad.

---

### `db` — Base de Datos

- Inicialización del pool SQLx PostgreSQL.
- Configuración de conexiones máximas y timeouts.

---

### `storage` — Almacenamiento de Objetos

- Abstracción sobre proveedores S3-compatibles.
- Soporta: local, S3, MinIO, Wasabi, DigitalOcean Spaces, Backblaze B2.

---

### `email` — Envío de Emails

- Integración con SMTP via `lettre`.
- Soporte TLS/SSL.

---

### `email_templates` — Plantillas de Email

- Plantillas HTML para emails transaccionales.
- Cargadas desde la base de datos (tabla `email_templates`).

---

### `push` — Push Notifications

- FCM (Firebase Cloud Messaging) para Android.
- APNs para iOS.
- VAPID/Web Push para navegadores.

---

### `sms` — SMS

- Integración con Twilio para envío de SMS.
- Usado para verificación de teléfono y 2FA.

---

### `i18n` — Internacionalización

- Carga de traducciones desde la base de datos.
- Soporte multi-idioma para mensajes del sistema.

---

### `crypto` — Criptografía

- Utilidades de hash y cifrado.
- AES-GCM para cifrado simétrico.
- HMAC para firmas.

---

### `sanitize` — Sanitización

- Limpieza de HTML con `ammonia` (previene XSS).
- Validación y sanitización de inputs de usuario.

---

### `validation` — Validación

- Validadores personalizados para campos comunes.
- Integración con el crate `validator`.

---

### `search` — Búsqueda

- Utilidades para búsqueda full-text en PostgreSQL.
- Construcción de queries de búsqueda.

---

### `site_config` — Configuración del Sitio

- Carga y caché de la configuración del sitio desde la base de datos.
- Actualización en tiempo real via Redis pub/sub.

---

### `points` — Sistema de Puntos

- Gestión del sistema de puntos/gamificación.
- Integración con AdMob para puntos por publicidad.

---

### `audit` — Auditoría

- Registro de acciones administrativas.
- Log de auditoría para cumplimiento.

---

### `internal_client` — Cliente HTTP Inter-Servicio

- Cliente HTTP para llamadas entre microservicios.
- Incluye el header `X-Internal-Key` para autenticación.

---

### `telemetry` — Telemetría

- Configuración de OpenTelemetry.
- Exportación de trazas via OTLP/gRPC.

---

### `test_helpers` — Helpers de Tests

- Utilidades para tests de integración.
- Setup de base de datos de test.
- Fixtures y factories.
