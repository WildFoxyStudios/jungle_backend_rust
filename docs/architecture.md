# Arquitectura del Sistema

## Visión General

Jungle Backend es una reescritura completa del backend PHP de la red social Jungle en **Rust**, usando una arquitectura de microservicios. Cada servicio es un binario independiente que se comunica a través de HTTP interno y un bus de eventos NATS.

## Diagrama de Arquitectura

```
                        ┌─────────────────────────────────────────────────────┐
                        │                   CLIENTES                          │
                        │   Web App · Mobile App · WebSocket · API externa    │
                        └──────────────────────┬──────────────────────────────┘
                                               │ HTTP / WS
                                               ▼
                        ┌─────────────────────────────────────────────────────┐
                        │              API GATEWAY  :8080                     │
                        │   Reverse proxy · Rate limiting · JWT validation    │
                        │   Swagger UI · OpenAPI JSON · /metrics              │
                        └──────┬──────────────────────────────────────────────┘
                               │ HTTP interno (service-to-service)
          ┌────────────────────┼────────────────────────────────────────────┐
          │                    │                                            │
          ▼                    ▼                                            ▼
   ┌─────────────┐    ┌─────────────────┐                        ┌──────────────────┐
   │auth-service │    │  user-service   │   ...13 servicios...   │  realtime-service│
   │   :3001     │    │    :3002        │                        │     :3012        │
   └──────┬──────┘    └────────┬────────┘                        └────────┬─────────┘
          │                    │                                           │
          └────────────────────┴───────────────────────────────────────────┘
                                               │
                               ┌───────────────┼───────────────┐
                               ▼               ▼               ▼
                        ┌────────────┐  ┌────────────┐  ┌────────────┐
                        │ PostgreSQL │  │   Redis    │  │    NATS    │
                        │    :5432   │  │   :6379    │  │   :4222    │
                        └────────────┘  └────────────┘  └────────────┘
                                                                │
                                               ┌───────────────┘
                                               ▼
                                        ┌────────────┐
                                        │   MinIO    │
                                        │ :9000/9001 │
                                        └────────────┘
```

## Servicios

| Servicio | Puerto | Responsabilidad |
|----------|--------|-----------------|
| `api-gateway` | 8080 | Punto de entrada público. Reverse proxy, rate limiting, routing, Swagger UI |
| `auth-service` | 3001 | Registro, login, JWT, refresh tokens, 2FA, OAuth social, sesiones |
| `user-service` | 3002 | Perfiles, búsqueda, follow/block/poke/mute, experiencia profesional |
| `post-service` | 3003 | Feed, posts, reacciones, comentarios, polls, hashtags, reels, ads |
| `messaging-service` | 3004 | Conversaciones, mensajes, typing, broadcasts, llamadas |
| `media-service` | 3005 | Subida de archivos, avatares, portadas, stories |
| `notification-service` | 3006 | Notificaciones CRUD, preferencias, push tokens, newsletter |
| `group-page-service` | 3007 | Páginas, grupos, eventos |
| `content-service` | 3008 | Blogs, foros, películas, juegos, páginas personalizadas |
| `commerce-service` | 3009 | Productos, pedidos, empleos, crowdfunding, ofertas, regalos, stickers |
| `admin-service` | 3010 | Dashboard, gestión de usuarios, reportes, configuración del sitio |
| `payment-service` | 3011 | 20 gateways de pago, wallet, suscripciones Pro/Creator |
| `realtime-service` | 3012 | WebSocket hub, presencia, typing, señalización WebRTC |
| `ai-service` | 3013 | Chat IA (OpenAI/Anthropic/Gemini), sugerencias de posts, descripción de imágenes |
| `jobs-runner` | — | 20 tareas en segundo plano (reemplazo de cron) |

## Stack Tecnológico

| Componente | Tecnología | Versión |
|------------|-----------|---------|
| Lenguaje | Rust | 1.86+ (edition 2024) |
| Framework HTTP | Axum | 0.8 |
| Base de datos | PostgreSQL | 16 |
| ORM/Query | SQLx | 0.8 |
| Caché | Redis | 7 |
| Bus de eventos | NATS | 2 (con JetStream) |
| Almacenamiento de objetos | MinIO (S3-compatible) | latest |
| Autenticación | JWT (jsonwebtoken 9) + Argon2 | — |
| Documentación API | utoipa + Swagger UI | 5/9 |
| Métricas | Prometheus | 0.13 |
| Trazas | OpenTelemetry (OTLP/gRPC) | 0.27 |
| Contenedores | Docker + Docker Compose | — |

## Principios de Diseño

- **Cursor-based pagination** en todos los endpoints de listado (no offset)
- **Eventos de dominio** via NATS para comunicación asíncrona entre servicios
- **Circuit breaker + retry con backoff** para llamadas a servicios externos
- **Dead Letter Queue (DLQ)** para mensajes NATS que fallan tras 3 intentos
- **Métricas Prometheus** en `/metrics` en cada servicio
- **Trazas OpenTelemetry** exportadas via OTLP/gRPC
- **Migraciones automáticas** via `sqlx::migrate!()` al arrancar cada servicio
- **SQLX offline mode** para compilar sin base de datos activa

## Flujo de una Petición Típica

```
1. Cliente → POST /v1/posts  (con Bearer token)
2. API Gateway valida JWT, aplica rate limit
3. Gateway hace proxy → post-service:3003/v1/posts
4. post-service inserta en PostgreSQL
5. post-service publica DomainEvent::PostCreated en NATS (events.post.created)
6. notification-service consume el evento → crea notificaciones
7. realtime-service consume el evento → push WebSocket a seguidores
8. Respuesta 201 Created regresa al cliente
```

## Comunicación Inter-Servicio

Los servicios se comunican de dos formas:

1. **HTTP síncrono** — via `internal_client` del crate `shared`. Usado cuando se necesita una respuesta inmediata.
2. **NATS asíncrono** — via `EventBus` trait. Usado para notificaciones, presencia, y propagación de eventos.

Ver [Event Bus](./event-bus.md) para la lista completa de eventos.
