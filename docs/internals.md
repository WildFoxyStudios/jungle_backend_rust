# Internals — Detalles de Implementación

Documentación de los detalles internos más importantes del sistema, extraídos directamente del código fuente.

---

## Autenticación — Detalles de Implementación

### Registro (`POST /v1/auth/register`)

1. Valida el request con `validator` (username, email, password, first_name)
2. Verifica unicidad de username/email (case-insensitive)
3. Hashea la contraseña con **Argon2id** (salt aleatorio via `OsRng`)
4. Inserta el usuario con `is_active = TRUE`
5. Genera access token JWT (15 min) y refresh token (UUID v4)
6. Almacena hash SHA-256 del refresh token en `sessions` (TTL: 30 días)
7. Publica evento `UserCreated` en NATS
8. Retorna `{ user, access_token, refresh_token, expires_in: 900 }`

### Login (`POST /v1/auth/login`)

El campo `identifier` acepta: username, email o número de teléfono.

**Soporte multi-hash legacy** (migración desde PHP):
- `$argon2*` → Argon2id (nuevo)
- `$2*` → bcrypt (legacy PHP) → se re-hashea a Argon2id automáticamente en el primer login
- 40 chars hex → SHA1 (muy legacy)
- 32 chars hex → MD5 (muy legacy)

Tras login exitoso:
- Actualiza `last_seen = NOW()` e `is_online = TRUE`
- Crea nueva sesión en `sessions`

### 2FA — TOTP

Implementación TOTP **desde cero** (sin librería externa):
- Secreto: 20 bytes aleatorios codificados en Base32
- Algoritmo: HMAC-SHA1 con contador de 30 segundos
- Ventana de tolerancia: ±1 período (30s antes y después)
- Códigos de respaldo: 10 códigos de 8 dígitos, almacenados como hash SHA-256

El endpoint `verify_2fa` acepta tanto códigos TOTP como códigos de respaldo. Los códigos de respaldo se eliminan tras su uso.

---

## Feed — Algoritmo de Ranking

El feed usa un **score de engagement temporal**:

```sql
(like_count + comment_count * 3 + share_count * 5)::float
/ GREATEST(EXTRACT(EPOCH FROM (NOW() - created_at)) / 3600, 1)
```

Los comentarios valen 3x y los shares 5x más que los likes. El score se divide por las horas desde la publicación (mínimo 1 hora para evitar división por cero).

**Orden final:**
1. Posts fijados (`is_pinned DESC`)
2. Posts impulsados (`is_boosted DESC`)
3. Score de engagement temporal

**Cache Redis:**
- Key: `feed:{user_id}:{cursor_id}:{filter}`
- TTL: 60 segundos
- Almacena solo los IDs de posts (no el contenido completo)
- En cache hit: batch load por IDs con `WHERE id = ANY($1)`

**Anuncios en el feed:**
- Se inyecta un anuncio cada 5 posts
- Selección aleatoria de anuncios activos con presupuesto > 0
- Cada impresión deduce $0.001 del presupuesto del anuncio

---

## Mensajería — Detalles

### Verificación de Membresía

Todos los endpoints de mensajes verifican que el usuario sea miembro activo de la conversación:

```sql
SELECT EXISTS(
  SELECT 1 FROM conversation_members
  WHERE conversation_id = $1 AND user_id = $2 AND is_active = TRUE
)
```

Si no es miembro → `403 Forbidden`.

### Envío de Mensajes (Transacción)

El envío de mensajes usa una **transacción PostgreSQL**:
1. `INSERT INTO messages` → obtiene el ID
2. `UPDATE conversations SET last_message_at = NOW()`
3. `COMMIT`

Esto garantiza que `last_message_at` siempre refleja el último mensaje real.

### Typing Indicator

El indicador de escritura usa Redis con TTL de 3 segundos:
```
SETEX typing:{conversation_id}:{user_id} 3 "1"
```

Además publica `DomainEvent::TypingStarted` en NATS para que el realtime-service lo retransmita via WebSocket.

### Reacciones a Mensajes (Toggle)

Las reacciones a mensajes tienen comportamiento toggle:
- Misma reacción → se elimina
- Reacción diferente → se actualiza
- Nueva reacción → se inserta

---

## Grafo Social — Detalles

### Follow con Confirmación

Si el usuario objetivo tiene `privacy_settings.confirm_followers = true`:
- El follow se crea con `status = 'pending'`
- No se publica el evento `FollowCreated`
- El usuario debe aceptar/rechazar desde `/v1/social/follow-requests`

Si el perfil es público:
- El follow se crea con `status = 'active'`
- Se publica `FollowCreated` inmediatamente

### Bloqueo

Al bloquear un usuario:
1. Se eliminan **todas** las relaciones de follow en ambas direcciones
2. Se inserta en `blocks`
3. Se publica `UserBlocked` en NATS

El bloqueo es bidireccional: si A bloquea a B, B tampoco puede ver el perfil de A (retorna 404).

---

## Validaciones

### Username
- Longitud: 3-50 caracteres
- Solo letras, números, guiones y guiones bajos
- Case-insensitive (almacenado en minúsculas para comparación)

### Password
- Mínimo 8 caracteres
- Debe contener al menos una letra y un número

### Contenido de Post
- Máximo 63,206 caracteres (límite de Twitter × 10)

### Comentario
- Mínimo 1 carácter, máximo 10,000

### Mensaje
- Mínimo 1 carácter, máximo 10,000
- O debe tener media adjunta

---

## Contadores Desnormalizados

Para evitar JOINs costosos en lecturas frecuentes, varios contadores se mantienen desnormalizados:

| Tabla | Columna | Actualizado cuando |
|-------|---------|-------------------|
| `posts` | `like_count` | Reacción añadida/eliminada |
| `posts` | `comment_count` | Comentario creado/eliminado |
| `posts` | `share_count` | Post compartido |
| `posts` | `view_count` | Post visualizado |
| `comments` | `like_count` | Reacción a comentario |
| `comments` | `reply_count` | Respuesta creada/eliminada |

Los contadores usan `GREATEST(count - 1, 0)` al decrementar para evitar valores negativos.

---

## Soft Delete

Los posts y usuarios usan soft delete:
- `deleted_at IS NULL` → activo
- `deleted_at IS NOT NULL` → eliminado

Todas las queries filtran `WHERE deleted_at IS NULL`. Los datos se mantienen en DB para auditoría y posible recuperación.

---

## Seguridad de Datos

### Passwords
- Nunca se almacenan en texto plano
- Argon2id para nuevos usuarios
- bcrypt/SHA1/MD5 para usuarios migrados (se re-hashean a Argon2id en el primer login)

### Refresh Tokens
- Almacenados como hash SHA-256 en `sessions`
- El token en texto plano solo existe en la respuesta HTTP y en la cookie httpOnly del cliente

### API Keys de IA
- Cifradas con AES-GCM antes de almacenarse en `ai_provider_config`
- Clave de cifrado en variable de entorno `AI_ENCRYPTION_KEY`
- Se muestran enmascaradas en las respuestas de admin

### Secretos TOTP
- Almacenados en texto plano en `users.two_factor_secret` (necesario para verificación)
- Los códigos de respaldo se almacenan como hash SHA-256

---

## Manejo de Errores en Eventos

Los errores de publicación en NATS **no fallan el request HTTP**:

```rust
// Patrón usado en todos los handlers
let _ = state.event_bus.publish(&DomainEvent::PostCreated { ... }).await;
// El .ok() o let _ = ignora el error intencionalmente
```

Esto garantiza que si NATS está caído, los usuarios siguen pudiendo crear posts, enviar mensajes, etc. Las notificaciones y actualizaciones en tiempo real pueden llegar tarde o no llegar, pero la operación principal no falla.

---

## Transacciones de Base de Datos

Se usan transacciones explícitas cuando múltiples operaciones deben ser atómicas:

```rust
let mut tx = state.db.begin().await?;
// operación 1
// operación 2
tx.commit().await?;
```

Ejemplos: envío de mensajes (insert + update conversation), creación de pedidos (insert order + update stock).
