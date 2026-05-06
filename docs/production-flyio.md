# Producción Backend en Fly.io

Esta guía deja el backend listo para despliegue en Fly.io con enfoque pragmático:
- `api-gateway` expuesto públicamente.
- microservicios en red privada Fly (`*.internal`).
- Postgres/Redis/NATS administrados (Fly o externos).

## 1) Topología recomendada

1. Una app Fly por binario Rust (`auth-service`, `user-service`, etc.).
2. `api-gateway` como único servicio público (`[[services]]` con puertos 80/443).
3. Backends internos sin exposición pública (`internal_port` solamente).
4. `DATABASE_URL`, `REDIS_URL`, `NATS_URL` compartidos por todos los servicios.

## 2) Configuración de routing interno en gateway

`api-gateway` resuelve upstreams por variables como:
- `AUTH_SERVICE_URL`
- `USER_SERVICE_URL`
- `POST_SERVICE_URL`
- ...

En Fly, usa DNS privado:

```env
AUTH_SERVICE_URL=http://auth-service.internal:3001
USER_SERVICE_URL=http://user-service.internal:3002
POST_SERVICE_URL=http://post-service.internal:3003
MESSAGING_SERVICE_URL=http://messaging-service.internal:3004
MEDIA_SERVICE_URL=http://media-service.internal:3005
NOTIFICATION_SERVICE_URL=http://notification-service.internal:3006
GROUP_PAGE_SERVICE_URL=http://group-page-service.internal:3007
CONTENT_SERVICE_URL=http://content-service.internal:3008
COMMERCE_SERVICE_URL=http://commerce-service.internal:3009
ADMIN_SERVICE_URL=http://admin-service.internal:3010
PAYMENT_SERVICE_URL=http://payment-service.internal:3011
REALTIME_SERVICE_URL=http://realtime-service.internal:3012
AI_SERVICE_URL=http://ai-service.internal:3013
```

## 3) `fly.toml` base (api-gateway)

Se incluye plantilla en `backend/fly.toml`. Ajusta:
- `app`
- `primary_region`
- `env` (dominios reales)
- `services.http_checks.path` (`/health`)

## 4) Secrets recomendados en Fly

```bash
fly secrets set \
  DATABASE_URL=... \
  REDIS_URL=... \
  NATS_URL=... \
  JWT_SECRET=... \
  JWT_REFRESH_SECRET=... \
  INTERNAL_SERVICE_KEY=... \
  ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com \
  FRONTEND_URL=https://app.example.com
```

## 5) Checklist de hardening

- `ALLOWED_ORIGINS` sin comodines.
- `PAYPAL_SANDBOX=false` y equivalentes en gateways productivos.
- `RUST_LOG` moderado (`info,sqlx=warn`).
- Health checks activos en todos los servicios.
- Alerta por ratio de 5xx, latencia p95 y fallos de webhook.

## 6) Flujo de release recomendado

1. Deploy backend interno (servicios) por lotes.
2. Deploy `api-gateway`.
3. Smoke tests: `/health`, login, feed, mensajes, pagos sandbox.
4. Monitoreo 15-30 min.
5. Si falla: rollback de la app afectada con `fly releases`.

## 7) Nota sobre migraciones

Ejecuta migraciones antes de enrutar tráfico nuevo (release phase o job dedicado) y valida:
- conteo de tablas esperado
- consultas críticas (auth, feed, wallet, admin) sin errores.

