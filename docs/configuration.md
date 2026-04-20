# Configuración y Entorno

## Variables de Entorno

Copia `.env.example` a `.env` y ajusta los valores antes de arrancar.

### Base de Datos

| Variable | Requerida | Descripción | Ejemplo |
|----------|-----------|-------------|---------|
| `DATABASE_URL` | ✅ | URL de conexión PostgreSQL | `postgresql://user:pass@localhost:5432/Jungle` |
| `DB_PASSWORD` | — | Contraseña para Docker Compose | `Jungle_dev_123` |

Soporta PostgreSQL local y Neon (serverless):
```
# Neon
DATABASE_URL=postgresql://user:pass@ep-xxx-pooler.us-east-1.aws.neon.tech/neondb?sslmode=require
```

### Redis

| Variable | Default | Descripción |
|----------|---------|-------------|
| `REDIS_URL` | `redis://127.0.0.1:6379` | URL de conexión Redis |

### JWT

| Variable | Requerida | Descripción |
|----------|-----------|-------------|
| `JWT_SECRET` | ✅ | Secreto para firmar access tokens (15 min de vida) |
| `JWT_REFRESH_SECRET` | — | Secreto para refresh tokens (fallback a `JWT_SECRET`) |

> **Producción**: usa strings aleatorios de al menos 64 caracteres.

### Servidor

| Variable | Default | Descripción |
|----------|---------|-------------|
| `SERVER_HOST` | `0.0.0.0` | Host de escucha |
| `SERVER_PORT` | `3000` | Puerto (cada servicio lo sobreescribe) |
| `GATEWAY_PORT` | `8080` | Puerto del API Gateway |
| `FRONTEND_URL` | `http://localhost:3000` | URL del frontend (para emails, redirects) |
| `ALLOWED_ORIGINS` | `http://localhost:3000,http://localhost:3001` | CORS origins (separados por coma) |

### NATS

| Variable | Default | Descripción |
|----------|---------|-------------|
| `NATS_URL` | `nats://127.0.0.1:4222` | URL del servidor NATS |

### Almacenamiento

| Variable | Default | Descripción |
|----------|---------|-------------|
| `STORAGE_PROVIDER` | `local` | `local`, `s3`, `minio`, `wasabi`, `spaces`, `backblaze` |
| `S3_ENDPOINT` | — | Endpoint S3/MinIO |
| `S3_BUCKET` | `Jungle` | Nombre del bucket |
| `S3_REGION` | `us-east-1` | Región |
| `S3_ACCESS_KEY` | — | Access key |
| `S3_SECRET_KEY` | — | Secret key |
| `S3_PUBLIC_URL` | — | URL pública para servir archivos |
| `MINIO_PASSWORD` | `minioadmin123` | Contraseña MinIO (Docker) |

### Email (SMTP)

| Variable | Descripción |
|----------|-------------|
| `SMTP_HOST` | Servidor SMTP (ej: `smtp.gmail.com`) |
| `SMTP_PORT` | Puerto (587 para TLS) |
| `SMTP_USERNAME` | Usuario SMTP |
| `SMTP_PASSWORD` | Contraseña SMTP |
| `SMTP_ENCRYPTION` | `tls` o `ssl` |
| `SMTP_FROM_EMAIL` | Email remitente |
| `SMTP_FROM_NAME` | Nombre remitente |

### SMS

| Variable | Descripción |
|----------|-------------|
| `SMS_PROVIDER` | `twilio` (único soportado actualmente) |
| `TWILIO_ACCOUNT_SID` | SID de cuenta Twilio |
| `TWILIO_AUTH_TOKEN` | Token de autenticación |
| `TWILIO_PHONE_FROM` | Número de teléfono origen |

### Push Notifications

| Variable | Descripción |
|----------|-------------|
| `FCM_PROJECT_ID` | ID del proyecto Firebase |
| `FCM_SERVICE_ACCOUNT_JSON` | JSON de cuenta de servicio FCM |
| `APNS_KEY_ID` | Key ID para APNs (iOS) |
| `APNS_TEAM_ID` | Team ID de Apple |
| `APNS_PRIVATE_KEY_PATH` | Ruta al archivo .p8 |
| `APNS_TOPIC` | Bundle ID de la app iOS |
| `VAPID_PRIVATE_KEY` | Clave privada VAPID (Web Push) |
| `VAPID_PUBLIC_KEY` | Clave pública VAPID |
| `VAPID_SUBJECT` | `mailto:admin@example.com` |

### AI Service

| Variable | Default | Descripción |
|----------|---------|-------------|
| `OPENAI_API_KEY` | — | API key de OpenAI (proveedor primario) |
| `OPENAI_MODEL` | `gpt-4o-mini` | Modelo de texto OpenAI |
| `OPENAI_IMAGE_MODEL` | `dall-e-3` | Modelo de imágenes OpenAI |
| `ANTHROPIC_API_KEY` | — | API key de Anthropic (fallback #1) |
| `ANTHROPIC_MODEL` | `claude-3-5-sonnet-20241022` | Modelo Anthropic |
| `GEMINI_API_KEY` | — | API key de Google Gemini (fallback #2) |
| `GEMINI_MODEL` | `gemini-1.5-flash` | Modelo Gemini |
| `GEMINI_IMAGE_MODEL` | `imagen-3.0-generate-001` | Modelo de imágenes Gemini |

### Inter-Service

| Variable | Descripción |
|----------|-------------|
| `INTERNAL_SERVICE_KEY` | Clave compartida para llamadas HTTP entre servicios |
| `AUTH_SERVICE_URL` | URL del auth-service (default: `http://127.0.0.1:3001`) |
| `USER_SERVICE_URL` | URL del user-service (default: `http://127.0.0.1:3002`) |
| *(etc.)* | Patrón: `{SERVICE_NAME}_URL` |

### Logging

| Variable | Default | Descripción |
|----------|---------|-------------|
| `RUST_LOG` | `info,sqlx=warn` | Filtro de logs (formato `tracing`) |
| `SQLX_OFFLINE` | `true` | Compilar sin DB activa (requiere `.sqlx/`) |

---

## Docker Compose

### Arrancar todo el stack

```bash
cp .env.example .env
docker compose up -d
```

### Servicios de infraestructura únicamente

```bash
docker compose up -d postgres redis nats minio
```

### Verificar salud

```bash
curl http://localhost:8080/health
```

### Puertos expuestos

| Servicio | Puerto |
|----------|--------|
| API Gateway | 8080 |
| PostgreSQL | 5432 |
| Redis | 6379 |
| NATS | 4222 (cliente), 8222 (HTTP monitoring) |
| MinIO API | 9000 |
| MinIO Console | 9001 |

---

## Dockerfile (Multi-stage Build)

El `Dockerfile` usa dos etapas:

1. **Builder** (`rust:bookworm`) — compila todos los binarios con `cargo build --release --workspace`. Optimiza el caché de capas copiando primero los `Cargo.toml` y creando stubs vacíos.

2. **Runtime** (`debian:bookworm-slim`) — imagen mínima con solo los binarios compilados, `ca-certificates`, `libssl3`, `curl`, y `tini` como PID 1.

El contenedor corre como usuario no-root `app` y expone un healthcheck en `http://localhost:${SERVER_PORT}/health`.

### Selección de binario

El `CMD` del servicio en `docker-compose.yml` determina qué binario ejecutar:
```yaml
command: ["auth-service"]   # ejecuta /usr/local/bin/auth-service
```

---

## Desarrollo Local

```bash
# Solo infraestructura
docker compose up -d postgres redis nats minio

# Copiar env
cp .env.example .env

# Ejecutar un servicio específico
cargo run -p auth-service

# Verificar compilación de todo el workspace
cargo check --workspace

# Ejecutar tests
cargo test --workspace

# Ejecutar jobs runner
cargo run -p jobs-runner
```

---

## Rate Limits (API Gateway)

| Endpoint | Límite |
|----------|--------|
| `/v1/auth/*` | 5 req / 15 min |
| `/v1/media/upload` | 20 req / min |
| `*/search*` | 30 req / min |
| `/v1/messages/*` | 60 req / min |
| Todo lo demás | 100 req / min |
