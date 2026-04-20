# API Gateway (Puerto 8080)

El API Gateway es el único punto de entrada público. Actúa como reverse proxy, aplica rate limiting y sirve la documentación Swagger.

---

## Routing

El gateway usa **prefix matching greedy** (prefijo más largo primero) para enrutar requests a los servicios upstream.

### Tabla de Routing

| Prefijo de Path | Servicio Upstream |
|-----------------|-------------------|
| `/v1/auth` | auth-service :3001 |
| `/v1/oauth` | auth-service :3001 |
| `/v1/translations` | auth-service :3001 |
| `/v1/config/public` | auth-service :3001 |
| `/v1/users` | user-service :3002 |
| `/v1/social` | user-service :3002 |
| `/v1/skills` | user-service :3002 |
| `/v1/posts` | post-service :3003 |
| `/v1/comments` | post-service :3003 |
| `/v1/feed` | post-service :3003 |
| `/v1/reels` | post-service :3003 |
| `/v1/search` | post-service :3003 |
| `/v1/ads` | post-service :3003 |
| `/v1/hashtags` | post-service :3003 |
| `/v1/memories` | post-service :3003 |
| `/v1/boosted` | post-service :3003 |
| `/v1/live` | post-service :3003 |
| `/v1/stories` | media-service :3005 |
| `/v1/media` | media-service :3005 |
| `/uploads` | media-service :3005 |
| `/v1/conversations` | messaging-service :3004 |
| `/v1/messages` | messaging-service :3004 |
| `/v1/broadcasts` | messaging-service :3004 |
| `/v1/calls` | messaging-service :3004 |
| `/v1/notifications` | notification-service :3006 |
| `/v1/announcements` | notification-service :3006 |
| `/v1/newsletter` | notification-service :3006 |
| `/v1/pages/custom` | content-service :3008 *(más específico que `/v1/pages`)* |
| `/v1/pages` | group-page-service :3007 |
| `/v1/groups` | group-page-service :3007 |
| `/v1/events` | group-page-service :3007 |
| `/v1/boosted/pages` | group-page-service :3007 |
| `/v1/blogs` | content-service :3008 |
| `/v1/forums` | content-service :3008 |
| `/v1/movies` | content-service :3008 |
| `/v1/games` | content-service :3008 |
| `/v1/products` | commerce-service :3009 |
| `/v1/orders` | commerce-service :3009 |
| `/v1/jobs` | commerce-service :3009 |
| `/v1/fundings` | commerce-service :3009 |
| `/v1/offers` | commerce-service :3009 |
| `/v1/gifts` | commerce-service :3009 |
| `/v1/stickers` | commerce-service :3009 |
| `/v1/payments` | payment-service :3011 |
| `/v1/admin` | admin-service :3010 |
| `/v1/ai` | ai-service :3013 |
| `/v1/presence` | realtime-service :3012 |
| `/ws` | realtime-service :3012 (WebSocket proxy) |

> El prefijo más largo tiene prioridad. Por eso `/v1/pages/custom` va a content-service aunque `/v1/pages` va a group-page-service.

---

## Rate Limiting

Implementado con **token bucket en Redis**. La clave de rate limit es `{ip}:{path_prefix}`.

| Path | Límite | Ventana |
|------|--------|---------|
| `/v1/auth/login`, `/v1/auth/register` | 10 req | 15 min |
| `/v1/auth/refresh` | 30 req | 1 min |
| `/v1/auth/*` (resto) | 15 req | 15 min |
| `/v1/media/upload` | 20 req | 1 min |
| `*/search*` | 30 req | 1 min |
| `/v1/messages/*`, `/v1/conversations/*` | 60 req | 1 min |
| Todo lo demás | 100 req | 1 min |

Cuando se supera el límite:
- Respuesta: `429 Too Many Requests`
- Header: `Retry-After: <segundos>`

### Implementación

```
1. INCR rate_limit:{ip}:{path}
2. Si es el primer incremento → EXPIRE {window_secs}
3. Si count > max → retorna 429 con TTL restante
4. Si count <= max → permite el request
```

---

## WebSocket Proxy

El gateway hace proxy de conexiones WebSocket al realtime-service:

```
Cliente → ws://localhost:8080/ws?token=<jwt>
         → realtime-service:3012/ws?token=<jwt>
```

El upgrade HTTP→WebSocket se maneja transparentemente.

---

## Swagger UI

Disponible en `http://localhost:8080/swagger-ui` cuando el gateway está corriendo.

El spec OpenAPI JSON está en `http://localhost:8080/api-docs/openapi.json`.

Documenta los 519 endpoints de los 15 servicios agrupados en tags.

---

## Health Check

```
GET /health
→ { "status": "healthy", "service": "api-gateway" }
```

---

## Configuración

Las URLs de los servicios upstream se configuran via variables de entorno:

```bash
AUTH_SERVICE_URL=http://auth-service:3001
USER_SERVICE_URL=http://user-service:3002
POST_SERVICE_URL=http://post-service:3003
MESSAGING_SERVICE_URL=http://messaging-service:3004
MEDIA_SERVICE_URL=http://media-service:3005
NOTIFICATION_SERVICE_URL=http://notification-service:3006
GROUP_PAGE_SERVICE_URL=http://group-page-service:3007
CONTENT_SERVICE_URL=http://content-service:3008
COMMERCE_SERVICE_URL=http://commerce-service:3009
ADMIN_SERVICE_URL=http://admin-service:3010
PAYMENT_SERVICE_URL=http://payment-service:3011
REALTIME_SERVICE_URL=http://realtime-service:3012
AI_SERVICE_URL=http://ai-service:3013
```

Si una variable no está definida, usa el default `http://127.0.0.1:{puerto}`.
