# API — Admin Service (Puerto 3010)

Todos los endpoints requieren `is_admin: true` en el JWT, excepto donde se indique.

---

## Dashboard

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/dashboard` | Estadísticas generales del sitio |
| GET | `/v1/admin/dashboard/charts` | Datos para gráficos (usuarios/posts por día) |
| GET | `/v1/admin/dashboard/top-countries` | Top países por usuarios registrados |
| GET | `/v1/admin/system-info` | Info del sistema (versión, uptime, DB size) |
| GET | `/v1/admin/health` | Health check detallado de todos los servicios |

---

## Gestión de Usuarios

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/users` | Listar todos los usuarios (paginado, filtrable) |
| GET | `/v1/admin/users/{id}` | Detalles completos de un usuario |
| PUT | `/v1/admin/users/{id}` | Actualizar datos del usuario |
| DELETE | `/v1/admin/users/{id}` | Eliminar usuario |
| POST | `/v1/admin/users/{id}/ban` | Banear usuario |
| POST | `/v1/admin/users/{id}/unban` | Desbanear usuario |
| POST | `/v1/admin/users/{id}/verify` | Verificar usuario (badge) |
| POST | `/v1/admin/users/{user_id}/make-admin` | Otorgar rol admin |
| POST | `/v1/admin/users/{user_id}/remove-admin` | Quitar rol admin |
| POST | `/v1/admin/users/{user_id}/make-pro` | Otorgar estado Pro |
| POST | `/v1/admin/users/{user_id}/remove-pro` | Quitar estado Pro |
| GET | `/v1/admin/users/{user_id}/permissions` | Permisos granulares del usuario |
| PUT | `/v1/admin/users/{user_id}/permissions` | Actualizar permisos granulares |
| POST | `/v1/admin/users/{user_id}/top-up` | Recargar wallet del usuario |
| DELETE | `/v1/admin/users/{user_id}/content` | Eliminar todo el contenido del usuario |
| POST | `/v1/admin/send-email` | Enviar email a un usuario |
| GET | `/v1/admin/pro-members` | Listar miembros Pro |
| GET | `/v1/admin/online-users` | Usuarios en línea ahora |
| GET | `/v1/admin/referrals` | Listar referidos |
| GET | `/v1/admin/fake-users` | Listar usuarios fake |
| POST | `/v1/admin/fake-users` | Crear usuario fake (para demos) |

---

## Reportes y Moderación

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/reports` | Listar reportes de usuarios |
| GET | `/v1/admin/reports/{id}` | Detalles de un reporte |
| POST | `/v1/admin/reports/{id}/resolve` | Resolver reporte |
| POST | `/v1/admin/reports/{id}/dismiss` | Descartar reporte |
| GET | `/v1/admin/moderation/posts` | Posts pendientes de aprobación |
| POST | `/v1/admin/moderation/posts/{id}/approve` | Aprobar post |
| POST | `/v1/admin/moderation/posts/{id}/reject` | Rechazar post |
| DELETE | `/v1/admin/posts/{id}` | Eliminar post definitivamente (hard delete) |
| GET | `/v1/admin/moderation/blogs` | Blogs pendientes de aprobación |
| POST | `/v1/admin/moderation/blogs/{id}/approve` | Aprobar blog |
| POST | `/v1/admin/moderation/blogs/{id}/reject` | Rechazar blog |
| GET | `/v1/admin/verifications` | Solicitudes de verificación pendientes |
| POST | `/v1/admin/verifications/{id}/approve` | Aprobar verificación |
| POST | `/v1/admin/verifications/{id}/reject` | Rechazar verificación |
| GET | `/v1/admin/banned-ips` | Listar IPs baneadas |
| POST | `/v1/admin/banned-ips` | Banear IP |
| DELETE | `/v1/admin/banned-ips/{id}` | Desbanear IP |

---

## Configuración del Sitio

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/config` | Listar toda la configuración |
| GET | `/v1/admin/config/{category}` | Configuración por categoría |
| PUT | `/v1/admin/config` | Actualizar configuración |
| GET | `/v1/admin/settings/{category}` | Configuración dinámica por categoría |
| PUT | `/v1/admin/settings/{category}` | Actualizar configuración por categoría |

---

## Pagos (Admin)

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/payments/stats` | Estadísticas de pagos |
| GET | `/v1/admin/payments/transactions` | Listar todas las transacciones |
| GET | `/v1/admin/payments/withdrawals` | Retiros pendientes |
| POST | `/v1/admin/payments/withdrawals/{id}/approve` | Aprobar retiro |
| POST | `/v1/admin/payments/withdrawals/{id}/reject` | Rechazar retiro |
| GET | `/v1/admin/payments/pro-plans` | Listar planes Pro |
| POST | `/v1/admin/payments/pro-plans` | Crear/actualizar plan Pro |
| GET | `/v1/admin/refunds` | Solicitudes de reembolso |
| POST | `/v1/admin/refunds/{id}/approve` | Aprobar reembolso |
| POST | `/v1/admin/refunds/{id}/reject` | Rechazar reembolso |
| GET | `/v1/admin/bank-receipts` | Comprobantes de transferencia bancaria |
| POST | `/v1/admin/bank-receipts/{id}/approve` | Aprobar comprobante |
| POST | `/v1/admin/bank-receipts/{id}/reject` | Rechazar comprobante |

---

## Gestión de Contenido

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/site-pages` | Listar páginas |
| DELETE | `/v1/admin/site-pages/{id}` | Eliminar página |
| GET | `/v1/admin/site-groups` | Listar grupos |
| DELETE | `/v1/admin/site-groups/{id}` | Eliminar grupo |
| GET | `/v1/admin/site-blogs` | Listar blogs |
| POST | `/v1/admin/site-blogs/{id}/approve` | Aprobar blog |
| DELETE | `/v1/admin/site-blogs/{id}` | Eliminar blog |
| GET | `/v1/admin/site-products` | Listar productos |
| DELETE | `/v1/admin/site-products/{id}` | Eliminar producto |
| GET | `/v1/admin/site-jobs` | Listar empleos |
| DELETE | `/v1/admin/site-jobs/{id}` | Eliminar empleo |
| GET | `/v1/admin/site-funding` | Listar campañas de crowdfunding |
| DELETE | `/v1/admin/site-funding/{id}` | Eliminar campaña |
| GET | `/v1/admin/site-events` | Listar eventos |
| DELETE | `/v1/admin/site-events/{id}` | Eliminar evento |
| GET | `/v1/admin/site-forums` | Listar foros |
| PUT | `/v1/admin/site-forums/{id}` | Actualizar foro |
| DELETE | `/v1/admin/site-forums/{id}` | Eliminar foro |
| GET | `/v1/admin/manage-posts` | Listar todos los posts |
| GET | `/v1/admin/stories` | Listar stories |
| POST | `/v1/admin/stories/{id}/hide` | Ocultar story |
| DELETE | `/v1/admin/stories/{id}` | Eliminar story |
| GET | `/v1/admin/offers` | Listar ofertas |
| DELETE | `/v1/admin/offers/{id}` | Eliminar oferta |
| GET | `/v1/admin/orders` | Listar pedidos |
| GET | `/v1/admin/reviews` | Listar reseñas |
| DELETE | `/v1/admin/reviews/{id}` | Eliminar reseña |

---

## Localización

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/languages` | Listar idiomas |
| POST | `/v1/admin/languages` | Crear idioma |
| PUT | `/v1/admin/languages/{id}` | Actualizar idioma |
| DELETE | `/v1/admin/languages/{id}` | Eliminar idioma |
| GET | `/v1/admin/translations` | Listar traducciones |
| POST | `/v1/admin/translations` | Crear/actualizar traducción |
| POST | `/v1/admin/translations/bulk` | Actualización masiva |
| DELETE | `/v1/admin/translations/{id}` | Eliminar traducción |

---

## Personalización

| Área | Endpoints |
|------|-----------|
| Categorías | CRUD `/v1/admin/categories` |
| Sub-categorías | CRUD `/v1/admin/sub-categories` |
| Posts de color | CRUD `/v1/admin/colored-posts` |
| Tipos de reacción | CRUD `/v1/admin/reaction-types` |
| Regalos | CRUD `/v1/admin/gifts` |
| Packs de stickers | CRUD `/v1/admin/sticker-packs` + `/v1/admin/stickers` |
| Plantillas de email | CRUD `/v1/admin/email-templates` |
| Campos de perfil | CRUD `/v1/admin/profile-fields` |
| Páginas personalizadas | CRUD `/v1/admin/pages` + `/v1/admin/pages/slug/{slug}` |
| Páginas de términos | GET/PUT `/v1/admin/terms-pages` |
| Géneros | CRUD `/v1/admin/genders` |
| Monedas | CRUD `/v1/admin/currencies` + toggle |

---

## Sistema y Operaciones

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/backups` | Listar backups |
| POST | `/v1/admin/backups/trigger` | Disparar backup manual |
| GET | `/v1/admin/newsletter/subscribers` | Suscriptores del newsletter |
| DELETE | `/v1/admin/newsletter/subscribers/{id}` | Eliminar suscriptor |
| POST | `/v1/admin/newsletter/send` | Enviar newsletter |
| GET | `/v1/admin/announcements` | Listar anuncios del sistema |
| POST | `/v1/admin/announcements` | Crear anuncio |
| PUT | `/v1/admin/announcements/{id}` | Actualizar anuncio |
| DELETE | `/v1/admin/announcements/{id}` | Eliminar anuncio |
| GET | `/v1/admin/invitations` | Listar invitaciones |
| POST | `/v1/admin/invitations` | Crear invitación |
| DELETE | `/v1/admin/invitations/{id}` | Eliminar invitación |
| GET | `/v1/admin/oauth-apps` | Listar apps OAuth |
| POST | `/v1/admin/oauth-apps/{id}/toggle` | Activar/desactivar app OAuth |
| DELETE | `/v1/admin/oauth-apps/{id}` | Eliminar app OAuth |
| GET | `/v1/admin/activities` | Log de actividad de usuarios |
| GET | `/v1/admin/audit-log` | Audit log de acciones admin |
| GET | `/v1/admin/ads` | Listar anuncios de usuarios |
| PUT | `/v1/admin/ads/{id}` | Actualizar anuncio |
| GET | `/v1/admin/user-ads` | Listar todos los anuncios |
| POST | `/v1/admin/user-ads/{id}/toggle` | Activar/desactivar anuncio |
| DELETE | `/v1/admin/user-ads/{id}` | Eliminar anuncio |
| GET | `/v1/admin/mass-notifications` | Listar notificaciones masivas |
| POST | `/v1/admin/mass-notifications/send` | Enviar notificación masiva (push) |
| POST | `/v1/admin/sitemap/generate` | Generar sitemap XML |
| GET | `/v1/admin/api-keys` | Listar API keys |
| POST | `/v1/admin/api-keys` | Crear API key |
| POST | `/v1/admin/api-keys/{id}/toggle` | Activar/desactivar API key |
| DELETE | `/v1/admin/api-keys/{id}` | Eliminar API key |
| GET | `/v1/admin/monetization` | Listar suscripciones de monetización |

---

## Dead Letter Queue (DLQ)

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/events/dlq` | Listar mensajes NATS fallidos |
| DELETE | `/v1/admin/events/dlq/{id}` | Descartar mensaje de la DLQ |
| POST | `/v1/admin/events/dlq/{id}/retry` | Reintentar mensaje de la DLQ |

---

## Almacenamiento

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/storage/config` | Listar proveedores de almacenamiento |
| POST | `/v1/admin/storage/config` | Crear proveedor (S3/MinIO/Wasabi/etc.) |
| PATCH | `/v1/admin/storage/config/{id}` | Actualizar proveedor |
| DELETE | `/v1/admin/storage/config/{id}` | Eliminar proveedor |
| POST | `/v1/admin/storage/config/{id}/test` | Probar conexión al proveedor |
| GET | `/v1/admin/permissions/catalog` | Catálogo de permisos granulares disponibles |

---

## Foros (Admin)

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/forum-sections` | Listar secciones |
| POST | `/v1/admin/forum-sections` | Crear sección |
| PUT | `/v1/admin/forum-sections/{id}` | Actualizar sección |
| DELETE | `/v1/admin/forum-sections/{id}` | Eliminar sección |
| POST | `/v1/admin/forums` | Crear foro |
| GET | `/v1/admin/forum-threads` | Listar hilos |
| DELETE | `/v1/admin/forum-threads/{id}` | Eliminar hilo |
| GET | `/v1/admin/forum-replies` | Listar respuestas |
| DELETE | `/v1/admin/forum-replies/{id}` | Eliminar respuesta |

---

## Películas y Juegos (Admin)

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/movies` | Listar películas |
| POST | `/v1/admin/manage-movies` | Crear película |
| PUT | `/v1/admin/manage-movies/{id}` | Actualizar película |
| POST | `/v1/admin/movies/{id}/approve` | Aprobar película |
| POST | `/v1/admin/movies/{id}/feature` | Destacar película |
| DELETE | `/v1/admin/movies/{id}` | Eliminar película |
| GET | `/v1/admin/games` | Listar juegos |
| POST | `/v1/admin/games` | Crear juego |
| POST | `/v1/admin/games/{id}/toggle` | Activar/desactivar juego |
| DELETE | `/v1/admin/games/{id}` | Eliminar juego |

---

## Configuración Avanzada

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/auto-settings` | Configuración automática (auto-amigos, auto-unión, auto-like) |
| PUT | `/v1/admin/auto-settings/auto-delete` | Configurar auto-eliminación de contenido |
| POST | `/v1/admin/auto-settings/friends` | Agregar usuario a auto-amigos |
| DELETE | `/v1/admin/auto-settings/friends/{id}` | Eliminar auto-amigo |
| POST | `/v1/admin/auto-settings/joins` | Agregar grupo a auto-unión |
| DELETE | `/v1/admin/auto-settings/joins/{id}` | Eliminar auto-unión |
| POST | `/v1/admin/auto-settings/likes` | Agregar página a auto-like |
| DELETE | `/v1/admin/auto-settings/likes/{id}` | Eliminar auto-like |
| GET | `/v1/admin/custom-code` | Código personalizado (header/footer HTML) |
| PUT | `/v1/admin/custom-code` | Actualizar código personalizado |
| GET | `/v1/admin/site-ads` | Anuncios del sitio (banners) |
| POST | `/v1/admin/site-ads` | Crear anuncio del sitio |
| PUT | `/v1/admin/site-ads/{id}` | Actualizar anuncio |
| DELETE | `/v1/admin/site-ads/{id}` | Eliminar anuncio |

---

## Live Streaming (Admin)

| Método | Ruta | Descripción |
|--------|------|-------------|
| GET | `/v1/admin/live-streams` | Listar transmisiones activas e historial |
| DELETE | `/v1/admin/live-streams/{id}` | Forzar fin de transmisión |
| GET | `/v1/admin/live/stats` | Estadísticas de live streaming |

---

## Email y Cronjobs

| Método | Ruta | Descripción |
|--------|------|-------------|
| POST | `/v1/admin/email-campaigns` | Crear campaña de email masivo |
| GET | `/v1/admin/cronjobs/status` | Estado de los background jobs |
| GET | `/v1/admin/cronjob-config` | Configuración de los cronjobs |
| PUT | `/v1/admin/cronjob-config/{name}` | Actualizar configuración de un cronjob |
