# Base de Datos

El sistema usa **PostgreSQL 16** como base de datos principal. Las migraciones se ejecutan automáticamente al arrancar cada servicio via `sqlx::migrate!()`.

---

## Migraciones

Las migraciones están en `backend/migrations/` y se ejecutan en orden numérico.

### Migraciones Originales (PHP → Rust)

| Archivo | Tablas Principales |
|---------|-------------------|
| `20250410000001_initial_schema.sql` | `users`, `sessions`, `backup_codes`, `login_attempts`, `banned_ips` |
| `20250410000002_social_graph.sql` | `follows`, `blocks`, `pokes`, `mutes`, `family_relations`, `experience`, `skills` |
| `20250410000003_posts_content.sql` | `posts`, `reactions`, `comments`, `polls`, `saved_posts`, `hidden_posts`, `hashtags` |
| `20250410000004_messaging.sql` | `conversations`, `conversation_members`, `messages`, `broadcasts` |
| `20250410000005_media_stories.sql` | `stories`, `story_media`, `story_views`, `albums`, `uploaded_media` |
| `20250410000006_notifications.sql` | `notifications`, `notification_settings` |
| `20250410000007_groups_pages_events.sql` | `categories`, `pages`, `groups`, `events`, `event_responses` |
| `20250410000008_content.sql` | `blogs`, `blog_comments`, `forum_sections`, `forums`, `threads`, `replies`, `movies`, `games` |
| `20250410000009_commerce.sql` | `products`, `reviews`, `orders`, `jobs`, `applications`, `fundings`, `donations`, `offers` |
| `20250410000010_payments_ads_config.sql` | `payment_transactions`, `withdrawals`, `ads`, `site_config`, `translations`, `reports` |
| `20250410000011_remaining.sql` | `calls`, `pro_subscriptions`, `creator_tiers`, `stickers`, `gifts`, `oauth_apps` |
| `20250410000012_admin_extras.sql` | Tablas adicionales de administración |
| `20250410000013_missing_php_tables.sql` | Tablas faltantes de la migración PHP |
| `20250410000014_admin_parity.sql` | Paridad con el panel admin PHP |
| `20250410000018_search_and_extras.sql` | Índices de búsqueda y extras |
| `20250410000019_push_tokens_and_extras.sql` | `push_tokens`, extras |
| `20250410000020_email_templates.sql` | `email_templates` |
| `20250410000021_new_features.sql` | Nuevas funcionalidades |
| `20250410000022_gap_parity.sql` | Paridad de gaps |
| `20250410000023_admin_advanced.sql` | Configuración avanzada de admin |
| `20250410000024_final_gaps.sql` | Gaps finales |

### Migraciones Nuevas (2026)

| Archivo | Descripción |
|---------|-------------|
| `20260418000001_ai_credits.sql` | Sistema de créditos de IA |
| `20260418000002_oauth_state.sql` | Estado OAuth para flujo de autorización |
| `20260418000003_audit_and_dlq.sql` | Log de auditoría y Dead Letter Queue |
| `20260418000004_endpoints_gaps.sql` | Gaps de endpoints |
| `20260418000005_onboarding_progress.sql` | Progreso de onboarding de usuarios |
| `20260418000006_newsletter_queue_and_cronruns.sql` | Cola de newsletter y registro de ejecuciones de cron |
| `20260418000007_users_extra_prefs.sql` | Preferencias adicionales de usuarios |

---

## Esquema Principal

### Tabla `users`

| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | `BIGSERIAL` | ID primario |
| `uuid` | `UUID` | UUID único del usuario |
| `username` | `VARCHAR(50)` | Nombre de usuario único |
| `email` | `VARCHAR(255)` | Email único |
| `phone` | `VARCHAR(20)` | Teléfono (opcional) |
| `password_hash` | `TEXT` | Hash Argon2id |
| `first_name` | `VARCHAR(100)` | Nombre |
| `last_name` | `VARCHAR(100)` | Apellido |
| `avatar` | `TEXT` | URL del avatar |
| `cover` | `TEXT` | URL de la portada |
| `bio` | `TEXT` | Biografía |
| `is_admin` | `BOOLEAN` | Rol administrador |
| `is_pro` | `BOOLEAN` | Estado Pro |
| `is_verified` | `BOOLEAN` | Verificado (badge) |
| `email_verified` | `BOOLEAN` | Email verificado |
| `phone_verified` | `BOOLEAN` | Teléfono verificado |
| `two_fa_enabled` | `BOOLEAN` | 2FA activo |
| `two_fa_secret` | `TEXT` | Secreto TOTP (cifrado) |
| `wallet_balance` | `DECIMAL(15,2)` | Saldo del wallet |
| `points` | `INTEGER` | Puntos de gamificación |
| `created_at` | `TIMESTAMPTZ` | Fecha de creación |
| `updated_at` | `TIMESTAMPTZ` | Última actualización |
| `last_seen` | `TIMESTAMPTZ` | Última vez visto |
| `banned` | `BOOLEAN` | Usuario baneado |
| `deleted_at` | `TIMESTAMPTZ` | Soft delete |

### Tabla `sessions`

| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | `BIGSERIAL` | ID primario |
| `user_id` | `BIGINT` | FK → users |
| `refresh_token_hash` | `TEXT` | Hash SHA-256 del refresh token |
| `device_info` | `JSONB` | Info del dispositivo |
| `ip_address` | `INET` | IP de la sesión |
| `expires_at` | `TIMESTAMPTZ` | Expiración |
| `created_at` | `TIMESTAMPTZ` | Creación |

### Tabla `posts`

| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | `BIGSERIAL` | ID primario |
| `user_id` | `BIGINT` | FK → users |
| `content` | `TEXT` | Contenido del post |
| `media` | `JSONB` | Array de media IDs |
| `privacy` | `VARCHAR(20)` | `public`, `friends`, `only_me` |
| `post_type` | `VARCHAR(20)` | `text`, `photo`, `video`, `reel`, `story` |
| `group_id` | `BIGINT` | FK → groups (opcional) |
| `page_id` | `BIGINT` | FK → pages (opcional) |
| `event_id` | `BIGINT` | FK → events (opcional) |
| `is_pinned` | `BOOLEAN` | Fijado en perfil |
| `is_boosted` | `BOOLEAN` | Impulsado |
| `scheduled_at` | `TIMESTAMPTZ` | Publicación programada |
| `status` | `VARCHAR(20)` | `published`, `draft`, `scheduled` |
| `reactions_count` | `INTEGER` | Contador de reacciones |
| `comments_count` | `INTEGER` | Contador de comentarios |
| `shares_count` | `INTEGER` | Contador de compartidos |
| `views_count` | `INTEGER` | Contador de vistas |
| `created_at` | `TIMESTAMPTZ` | Creación |

### Tabla `conversations`

| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | `BIGSERIAL` | ID primario |
| `type` | `VARCHAR(20)` | `direct`, `group` |
| `name` | `VARCHAR(255)` | Nombre (grupos) |
| `avatar` | `TEXT` | Avatar (grupos) |
| `color` | `VARCHAR(20)` | Color de la conversación |
| `last_message_id` | `BIGINT` | FK → messages |
| `created_at` | `TIMESTAMPTZ` | Creación |

### Tabla `payment_transactions`

| Columna | Tipo | Descripción |
|---------|------|-------------|
| `id` | `BIGSERIAL` | ID primario |
| `user_id` | `BIGINT` | FK → users |
| `provider` | `VARCHAR(50)` | Proveedor de pago |
| `provider_ref` | `TEXT` | Referencia del proveedor |
| `amount` | `DECIMAL(15,2)` | Monto |
| `currency` | `VARCHAR(10)` | Moneda |
| `status` | `VARCHAR(20)` | `pending`, `completed`, `failed`, `refunded` |
| `payment_type` | `VARCHAR(50)` | `pro_subscription`, `wallet_topup`, etc. |
| `metadata` | `JSONB` | Datos adicionales |
| `created_at` | `TIMESTAMPTZ` | Creación |

---

## Índices Importantes

```sql
-- Búsqueda de usuarios
CREATE INDEX idx_users_username ON users(username);
CREATE INDEX idx_users_email ON users(email);

-- Feed (cursor pagination)
CREATE INDEX idx_posts_user_id_created ON posts(user_id, created_at DESC);
CREATE INDEX idx_posts_created_at ON posts(created_at DESC) WHERE status = 'published';

-- Mensajes (cursor pagination)
CREATE INDEX idx_messages_conversation_created ON messages(conversation_id, created_at DESC);

-- Notificaciones
CREATE INDEX idx_notifications_recipient ON notifications(recipient_id, created_at DESC);
CREATE INDEX idx_notifications_unread ON notifications(recipient_id) WHERE read = false;
```

---

## Conexión

```bash
# Conectar con psql
psql postgresql://Jungle:Jungle_dev_123@localhost:5432/Jungle

# Ver tablas
\dt

# Ver migraciones aplicadas
SELECT * FROM _sqlx_migrations ORDER BY installed_on;
```

---

## Migración desde MySQL

Ver [Migración MySQL → PostgreSQL](./migration.md) para migrar datos desde la instalación PHP original.
