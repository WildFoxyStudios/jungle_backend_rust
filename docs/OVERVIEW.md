# Jungle Backend — Resumen Ejecutivo

## ¿Qué es esto?

Reescritura completa del backend de la red social **Jungle** (originalmente en PHP) en **Rust**, usando arquitectura de microservicios. El resultado es un backend de producción listo para escalar, con 519 endpoints documentados, 15 servicios independientes y 20 gateways de pago integrados.

---

## Números Clave

| Métrica | Valor |
|---------|-------|
| Endpoints REST | 519 |
| Microservicios | 15 |
| Gateways de pago | 20 |
| Proveedores de IA | 3 (OpenAI, Anthropic, Gemini) |
| Proveedores OAuth social | 14 |
| Background jobs | 20 |
| Migraciones SQL | 28 |
| Tests unitarios | 34 |
| Líneas de código (aprox.) | ~25,000 |

---

## Funcionalidades Incluidas

### Red Social Core
- Feed personalizado con algoritmo de ranking por engagement
- Posts (texto, foto, video, audio, reels, live streaming)
- Reacciones configurables (like, love, haha, wow, sad, angry)
- Comentarios y respuestas anidadas
- Historias (stories) con expiración automática a 24h
- Hashtags trending (actualizado cada 15 min)
- Búsqueda global (usuarios, posts, páginas, grupos, hashtags, blogs, productos)
- Memorias "En este día"

### Grafo Social
- Follow/unfollow con aprobación opcional (perfiles privados)
- Bloqueo, silencio, toque (poke)
- Relaciones de familia
- Solicitudes de seguimiento pendientes

### Mensajería
- Conversaciones directas y grupales
- Mensajes de texto, imagen, video, audio, sticker, GIF, ubicación
- Respuestas, reenvío, favoritos, mensajes fijados
- Indicador de escritura en tiempo real (Redis TTL 3s)
- Broadcasts (listas de difusión)
- Llamadas de audio/video (señalización WebRTC + Agora)

### Tiempo Real
- WebSocket con hub de conexiones (DashMap, broadcast channel 256)
- Presencia online/offline
- Relay de eventos NATS → WebSocket

### Contenido
- Blogs con editor de imágenes
- Foros con secciones, hilos y respuestas
- Películas y juegos
- Páginas personalizadas (CMS básico)

### Comercio
- Marketplace de productos con carrito y pedidos
- Empleos con aplicaciones
- Crowdfunding con donaciones
- Ofertas geolocalizadas
- Regalos virtuales y stickers

### Pagos
- 20 gateways: Stripe, PayPal, Paystack, Flutterwave, Razorpay, Coinbase, Braintree, Iyzipay, Cashfree, YooMoney, aamarPay, Fortumo, 2Checkout, CoinPayments, PayFast, Paysera, SecurionPay, N-Genius, PayPro Bitcoin, Bank Transfer
- Wallet interno con transferencias entre usuarios
- Suscripciones Pro
- Suscripciones Creator (modo Patreon)
- Webhooks con verificación de firma

### IA
- Generación de posts y blogs
- Generación de imágenes (DALL-E 3, Imagen 3)
- Descripción de imágenes (accesibilidad)
- Chat conversacional
- Sistema de créditos por usuario
- Configuración dinámica de proveedores desde el admin
- API keys cifradas con AES-GCM

### Administración
- Dashboard con estadísticas y gráficos
- Gestión completa de usuarios (ban, verify, pro, admin)
- Moderación de contenido (posts, blogs, verificaciones)
- Configuración del sitio por categorías
- Gestión de pagos y retiros
- Localización (idiomas + traducciones)
- Personalización (categorías, reacciones, regalos, stickers, campos de perfil)
- Audit log de todas las acciones admin
- Dead Letter Queue (DLQ) con retry manual
- Configuración de almacenamiento (S3/MinIO/Wasabi/Spaces/Backblaze)
- Gestión de cronjobs desde el panel
- Campañas de email masivo
- Notificaciones masivas push
- Generación de sitemap
- Usuarios fake para demos

### Seguridad
- JWT (access 15min + refresh 30 días, httpOnly cookie)
- Argon2id para passwords (con migración automática desde bcrypt/SHA1/MD5)
- 2FA TOTP implementado desde cero (sin dependencias externas)
- Rate limiting por IP en Redis (token bucket)
- IPs baneadas verificadas en cada request
- Sanitización HTML con ammonia (previene XSS)

### Infraestructura
- Docker Compose con healthchecks
- Dockerfile multi-stage (builder + runtime mínimo)
- Migraciones automáticas al arrancar
- Prometheus metrics en `/metrics` en cada servicio
- OpenTelemetry (OTLP/gRPC) para trazas distribuidas
- Circuit breaker + retry con backoff exponencial
- Dead Letter Queue para eventos NATS fallidos
- Graceful shutdown

---

## Stack Tecnológico

```
Rust 1.86+ (edition 2024)
├── Axum 0.8          — HTTP framework
├── SQLx 0.8          — PostgreSQL async
├── Redis 7           — Cache, rate limiting, presencia
├── NATS 2            — Event bus (JetStream)
├── MinIO             — Object storage (S3-compatible)
├── Prometheus 0.13   — Métricas
└── OpenTelemetry 0.27 — Trazas distribuidas
```

---

## Arquitectura

```
Internet → API Gateway :8080 → 15 microservicios (:3001-:3013)
                                        ↓
                    PostgreSQL 16 + Redis 7 + NATS + MinIO
```

Cada servicio es un binario Rust independiente. Se comunican via:
1. HTTP síncrono (para datos que necesitan respuesta inmediata)
2. NATS asíncrono (para notificaciones, presencia, eventos)

---

## Documentación Incluida

- [Arquitectura completa](./architecture.md)
- [API Gateway y routing](./api-gateway.md)
- [Configuración y variables de entorno](./configuration.md)
- [Autenticación JWT/2FA/OAuth](./auth.md)
- [API de cada servicio](./api/) — 13 documentos
- [Modelos de datos con ejemplos JSON](./data-models.md)
- [Detalles de implementación internos](./internals.md)
- [Event Bus NATS](./event-bus.md)
- [20 Background Jobs](./jobs.md)
- [20 Payment Gateways](./payment-gateways.md)
- [Base de datos y migraciones](./database.md)
- [Observabilidad](./observability.md)
- [Resiliencia](./resilience.md)
- [Migración desde MySQL](./migration.md)
