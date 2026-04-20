# Jungle Backend — Documentación

Bienvenido al índice de documentación del backend de Jungle.

> Para una visión rápida del proyecto, lee primero el [Resumen Ejecutivo](./OVERVIEW.md).

## Índice

### Empezar Aquí

| Documento | Descripción |
|-----------|-------------|
| [**Resumen Ejecutivo**](./OVERVIEW.md) | Qué incluye, números clave, funcionalidades |
| [Arquitectura](./architecture.md) | Diagrama de servicios, stack tecnológico, flujo de requests |
| [API Gateway](./api-gateway.md) | Routing completo, rate limiting, WebSocket proxy |
| [Configuración & Entorno](./configuration.md) | Variables de entorno, Docker, despliegue |
| [Autenticación](./auth.md) | JWT, refresh tokens, 2FA TOTP, OAuth social (14 proveedores) |

### APIs por Servicio

| Documento | Puerto | Endpoints | Descripción |
|-----------|--------|-----------|-------------|
| [Auth Service](./api/auth-service.md) | 3001 | ~30 | Registro, login, sesiones, 2FA, OAuth apps |
| [User Service](./api/user-service.md) | 3002 | ~60 | Perfiles, grafo social, profesional, configuración |
| [Post Service](./api/post-service.md) | 3003 | ~70 | Feed, posts, reacciones, comentarios, reels, live |
| [Messaging Service](./api/messaging-service.md) | 3004 | ~35 | Conversaciones, mensajes, llamadas, broadcasts |
| [Media Service](./api/media-service.md) | 3005 | ~20 | Subida de archivos, transformación, stories |
| [Notification Service](./api/notification-service.md) | 3006 | ~15 | Notificaciones, push tokens, newsletter |
| [Group & Page Service](./api/group-page-service.md) | 3007 | ~65 | Páginas, grupos, eventos |
| [Content Service](./api/content-service.md) | 3008 | ~40 | Blogs, foros, películas, juegos |
| [Commerce Service](./api/commerce-service.md) | 3009 | ~55 | Productos, pedidos, empleos, crowdfunding, regalos |
| [Payment Service](./api/payment-service.md) | 3011 | ~25 | Pagos, wallet, suscripciones Pro/Creator, webhooks |
| [Admin Service](./api/admin-service.md) | 3010 | ~150 | Dashboard, moderación, configuración completa |
| [Realtime Service](./api/realtime-service.md) | 3012 | WebSocket | Hub WS, presencia, relay de eventos NATS |
| [AI Service](./api/ai-service.md) | 3013 | ~15 | Generación texto/imágenes, créditos, admin de proveedores |

### Referencia Técnica

| Documento | Descripción |
|-----------|-------------|
| [Modelos de Datos](./data-models.md) | Estructuras reales de request/response con ejemplos JSON completos |
| [Shared Crate](./shared-crate.md) | Módulos comunes: auth, errores, eventos, métricas, paginación |
| [Internals](./internals.md) | Algoritmos, transacciones, validaciones, detalles de implementación |
| [Event Bus (NATS)](./event-bus.md) | 28 eventos de dominio, sujetos NATS, DLQ |
| [Background Jobs](./jobs.md) | 20 tareas en segundo plano con frecuencias |
| [Payment Gateways](./payment-gateways.md) | 20 proveedores de pago con variables de entorno |
| [Base de Datos](./database.md) | 28 migraciones SQL, esquema de tablas principales |
| [Observabilidad](./observability.md) | Prometheus, OpenTelemetry, logs estructurados |
| [Resiliencia](./resilience.md) | Circuit breaker, retry con backoff, DLQ, rate limiting |
| [Migración MySQL → PostgreSQL](./migration.md) | Herramienta de migración desde instalación PHP |
