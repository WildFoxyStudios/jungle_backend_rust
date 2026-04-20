# Modelos de Datos

Documentación de las estructuras de datos reales usadas en requests, responses y la base de datos.

---

## Usuario

### `User` (modelo interno completo)

Mapeado directamente desde la tabla `users`. Nunca se expone directamente al cliente.

| Campo | Tipo Rust | Tipo DB | Descripción |
|-------|-----------|---------|-------------|
| `id` | `i64` | `BIGSERIAL` | ID primario |
| `uuid` | `Uuid` | `UUID` | UUID único |
| `username` | `String` | `VARCHAR(50)` | Nombre de usuario (único, case-insensitive) |
| `email` | `String` | `VARCHAR(255)` | Email (único, almacenado en minúsculas) |
| `phone_number` | `Option<String>` | `VARCHAR(20)` | Teléfono (opcional) |
| `password_hash` | `String` | `TEXT` | Hash Argon2id (o bcrypt/SHA1/MD5 legacy) |
| `first_name` | `String` | `VARCHAR(100)` | Nombre |
| `last_name` | `String` | `VARCHAR(100)` | Apellido |
| `avatar` | `String` | `TEXT` | URL del avatar |
| `cover` | `String` | `TEXT` | URL de la portada |
| `about` | `String` | `TEXT` | Biografía |
| `gender` | `String` | `VARCHAR(20)` | Género |
| `birthday` | `Option<Date>` | `DATE` | Fecha de nacimiento |
| `country_id` | `Option<i32>` | `INTEGER` | ID del país |
| `city` | `String` | `TEXT` | Ciudad/ubicación |
| `address` | `String` | `TEXT` | Dirección |
| `website` | `String` | `TEXT` | Sitio web |
| `school` | `String` | `TEXT` | Escuela/universidad |
| `working` | `String` | `TEXT` | Empresa donde trabaja |
| `working_link` | `String` | `TEXT` | URL de la empresa |
| `language` | `String` | `VARCHAR(10)` | Idioma preferido |
| `is_active` | `bool` | `BOOLEAN` | Cuenta verificada/activa |
| `is_admin` | `bool` | `BOOLEAN` | Rol administrador |
| `is_pro` | `i16` | `SMALLINT` | Nivel Pro (0=free, 1=pro) |
| `is_verified` | `bool` | `BOOLEAN` | Badge de verificación |
| `email_verified` | `bool` | `BOOLEAN` | Email verificado |
| `phone_verified` | `bool` | `BOOLEAN` | Teléfono verificado |
| `email_code` | `String` | `TEXT` | Código de verificación de email |
| `privacy_settings` | `Value` | `JSONB` | Configuración de privacidad |
| `notification_settings` | `Value` | `JSONB` | Configuración de notificaciones |
| `balance` | `Decimal` | `DECIMAL(15,2)` | Saldo de puntos/créditos |
| `wallet` | `Decimal` | `DECIMAL(15,2)` | Saldo del wallet de pagos |
| `points` | `i64` | `BIGINT` | Puntos de gamificación |
| `social_logins` | `Value` | `JSONB` | Proveedores OAuth vinculados |
| `lat` | `Option<f64>` | `DOUBLE PRECISION` | Latitud |
| `lng` | `Option<f64>` | `DOUBLE PRECISION` | Longitud |
| `last_seen` | `Option<OffsetDateTime>` | `TIMESTAMPTZ` | Última actividad |
| `is_online` | `bool` | `BOOLEAN` | En línea ahora |
| `two_factor_enabled` | `bool` | `BOOLEAN` | 2FA activo |
| `two_factor_method` | `Option<String>` | `VARCHAR(20)` | Método 2FA (`totp`) |
| `two_factor_secret` | `Option<String>` | `TEXT` | Secreto TOTP |
| `deleted_at` | `Option<OffsetDateTime>` | `TIMESTAMPTZ` | Soft delete |
| `created_at` | `OffsetDateTime` | `TIMESTAMPTZ` | Creación |
| `updated_at` | `OffsetDateTime` | `TIMESTAMPTZ` | Última actualización |
| `is_fake` | `bool` | `BOOLEAN` | Usuario de prueba/fake |
| `monetization_enabled` | `bool` | `BOOLEAN` | Monetización activada |
| `subscription_price` | `Decimal` | `DECIMAL(15,2)` | Precio de suscripción creator |
| `is_live` | `Option<bool>` | `BOOLEAN` | Transmitiendo en vivo |
| `live_stream_id` | `Option<String>` | `TEXT` | ID del stream activo |
| `monetization_settings` | `Value` | `JSONB` | Configuración de monetización |
| `android_device_id` | `Option<String>` | `TEXT` | Device ID Android |
| `ios_device_id` | `Option<String>` | `TEXT` | Device ID iOS |
| `android_notification_id` | `Option<String>` | `TEXT` | FCM token Android |
| `ios_notification_id` | `Option<String>` | `TEXT` | APNs token iOS |
| `social_links` | `Value` | `JSONB` | Links de redes sociales |
| `start_up_info` | `bool` | `BOOLEAN` | Onboarding info completado |
| `startup_image` | `bool` | `BOOLEAN` | Onboarding imagen completado |
| `startup_follow` | `bool` | `BOOLEAN` | Onboarding seguir completado |

---

### `AuthUserResponse` (respuesta autenticada)

Devuelto en login, register y `GET /v1/auth/me`. Incluye campos privados del usuario autenticado.

```json
{
  "id": 42,
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "username": "john_doe",
  "email": "john@example.com",
  "first_name": "John",
  "last_name": "Doe",
  "name": "John Doe",
  "avatar": "https://cdn.example.com/avatars/john.jpg",
  "cover": "https://cdn.example.com/covers/john.jpg",
  "about": "Developer & coffee lover",
  "gender": "male",
  "birthday": "1990-01-15",
  "location": "Madrid",
  "website": "https://johndoe.com",
  "school": "Universidad Complutense",
  "working": "Acme Corp",
  "is_verified": true,
  "is_pro": 1,
  "is_admin": false,
  "two_factor_enabled": false,
  "email_verified": true
}
```

---

### `PublicUser` (perfil público)

Devuelto en `GET /v1/users/{username}`. No incluye email ni datos privados.

```json
{
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "username": "john_doe",
  "first_name": "John",
  "last_name": "Doe",
  "name": "John Doe",
  "avatar": "https://cdn.example.com/avatars/john.jpg",
  "cover": "https://cdn.example.com/covers/john.jpg",
  "about": "Developer & coffee lover",
  "is_verified": true,
  "is_pro": 1,
  "is_online": false
}
```

La respuesta completa de `GET /v1/users/{username}` incluye además:

```json
{
  "data": {
    "user": { ... },
    "follower_count": 1250,
    "following_count": 340,
    "is_following": false,
    "is_following_me": true
  }
}
```

---

### `PublicUserRow` (mini perfil para embeds)

Usado dentro de posts, comentarios, etc. para mostrar info del autor.

```json
{
  "uuid": "...",
  "username": "john_doe",
  "first_name": "John",
  "last_name": "Doe",
  "avatar": "...",
  "cover": "...",
  "about": "...",
  "is_verified": true,
  "is_pro": 0
}
```

---

### Popover de Usuario

`GET /v1/users/{username}/popover` — respuesta optimizada para hover cards (70% más pequeña que el perfil completo):

```json
{
  "data": {
    "id": 42,
    "username": "john_doe",
    "first_name": "John",
    "last_name": "Doe",
    "avatar": "...",
    "about": "Developer",
    "is_verified": true,
    "follower_count": 1250,
    "following_count": 340,
    "post_count": 87,
    "is_following": false
  }
}
```

---

## Post

### `PostRow` (modelo de post)

```json
{
  "id": 123,
  "uuid": "...",
  "user_id": 42,
  "parent_id": null,
  "content": "Hello world!",
  "post_type": "text",
  "media": [],
  "privacy": "everyone",
  "feeling": "happy",
  "location": "Madrid",
  "is_pinned": false,
  "is_boosted": false,
  "is_reel": false,
  "like_count": 15,
  "comment_count": 3,
  "share_count": 1,
  "view_count": 200,
  "created_at": "2026-04-18T10:00:00Z",
  "updated_at": "2026-04-18T10:00:00Z"
}
```

### Tipos de Post (`post_type`)

| Valor | Descripción |
|-------|-------------|
| `text` | Post de solo texto |
| `media` | Post con imágenes/videos |
| `reel` | Reel (video corto) |
| `link` | Post con enlace |
| `poll` | Encuesta |
| `live` | Transmisión en vivo |

### Valores de Privacidad (`privacy`)

| Valor | Descripción |
|-------|-------------|
| `everyone` | Público (todos) |
| `friends` | Solo seguidores |
| `only_me` | Solo yo |
| `custom` | Lista personalizada |

### Respuesta del Feed

El feed incluye el post + info del autor + anuncios cada 5 posts:

```json
{
  "data": [
    {
      "id": 123,
      "content": "...",
      "publisher": {
        "uuid": "...",
        "username": "john_doe",
        "avatar": "...",
        "is_verified": true
      },
      "is_ad": false
    },
    {
      "id": 100,
      "content": "Sponsored content",
      "is_ad": true,
      "ad_id": 5
    }
  ],
  "meta": { "cursor": "122", "has_more": true }
}
```

### Algoritmo del Feed

El feed usa un ranking híbrido:
1. Posts fijados (`is_pinned = true`) primero
2. Posts impulsados (`is_boosted = true`) segundo
3. Score de engagement: `(likes + comments*3 + shares*5) / horas_desde_publicacion`

Fuentes del feed:
- Posts propios del usuario
- Posts de usuarios seguidos
- Posts de páginas con like
- Posts de grupos unidos

Filtros aplicados:
- Excluye posts de usuarios bloqueados
- Excluye posts ocultos por el usuario
- Excluye posts `only_me` de otros usuarios
- Solo posts aprobados (`is_approved = true`)

Cache Redis: IDs del feed cacheados 60 segundos por `user_id:cursor:filter`.

---

## Comentario

### `CommentRow`

```json
{
  "id": 456,
  "user_id": 42,
  "post_id": 123,
  "parent_id": null,
  "content": "Gran post!",
  "media": [],
  "like_count": 2,
  "reply_count": 1,
  "created_at": "2026-04-18T10:05:00Z"
}
```

Los comentarios de primer nivel tienen `parent_id: null`. Las respuestas tienen `parent_id` apuntando al comentario padre.

Los contadores `comment_count` en posts y `reply_count` en comentarios se actualizan de forma **desnormalizada** (sin JOIN) para rendimiento.

---

## Mensaje

### `MessageWithSender`

Respuesta completa de un mensaje con info del remitente embebida:

```json
{
  "id": 789,
  "conversation_id": 10,
  "sender_id": 42,
  "sender_username": "john_doe",
  "sender_first_name": "John",
  "sender_last_name": "Doe",
  "sender_avatar": "...",
  "content": "Hola!",
  "message_type": "text",
  "media": [],
  "reply_to_id": null,
  "forwarded_from": null,
  "is_pinned": false,
  "is_favorited": false,
  "created_at": "2026-04-18T10:00:00Z"
}
```

### `MessageResponse` (con preview de respuesta)

```json
{
  "message": { ... },
  "reply_to": {
    "id": 788,
    "sender_id": 43,
    "sender_username": "jane_doe",
    "content": "Mensaje original",
    "message_type": "text"
  }
}
```

### Tipos de Mensaje (`message_type`)

| Valor | Descripción |
|-------|-------------|
| `text` | Texto plano |
| `image` | Imagen |
| `video` | Video |
| `audio` | Audio/voz |
| `sticker` | Sticker |
| `gif` | GIF animado |
| `location` | Ubicación geográfica |
| `file` | Archivo adjunto |

---

## Reacción

Las reacciones usan una tabla polimórfica con `target_type`:

| `target_type` | Descripción |
|---------------|-------------|
| `post` | Reacción a un post |
| `comment` | Reacción a un comentario |
| `message` | Reacción a un mensaje |

Tipos de reacción por defecto: `like`, `love`, `haha`, `wow`, `sad`, `angry`

Los tipos son configurables desde el panel de administración.

Las reacciones a mensajes son **toggle**: si el usuario reacciona con el mismo emoji, se elimina. Si reacciona con uno diferente, se actualiza.

---

## Seguimiento (Follow)

### `FollowUser` (usuario en lista de seguidores/seguidos)

```json
{
  "id": 42,
  "uuid": "...",
  "username": "john_doe",
  "first_name": "John",
  "last_name": "Doe",
  "avatar": "...",
  "is_verified": true,
  "is_pro": 0
}
```

### Estados del Follow

| Estado | Descripción |
|--------|-------------|
| `active` | Seguimiento activo |
| `pending` | Esperando aprobación (perfil privado) |

Si el usuario objetivo tiene `privacy_settings.confirm_followers = true`, el follow queda en estado `pending` hasta que sea aceptado.

---

## Paginación

Todas las listas usan cursor-based pagination:

```json
{
  "data": [...],
  "meta": {
    "cursor": "122",
    "has_more": true,
    "total": null
  }
}
```

El cursor es el ID del último elemento. Para obtener la siguiente página:
```
GET /v1/feed?cursor=122&limit=20
```

Límites: mínimo 1, máximo 100, default 20.

---

## Errores

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid input",
    "details": [
      { "field": "email", "message": "Invalid email format" },
      { "field": "password", "message": "Password too short" }
    ]
  }
}
```

| Código | HTTP | Descripción |
|--------|------|-------------|
| `BAD_REQUEST` | 400 | Parámetros inválidos |
| `UNAUTHORIZED` | 401 | Token faltante, inválido o expirado |
| `FORBIDDEN` | 403 | Sin permisos |
| `NOT_FOUND` | 404 | Recurso no encontrado |
| `CONFLICT` | 409 | Recurso ya existe (ej: username duplicado) |
| `VALIDATION_ERROR` | 422 | Errores de validación de campos |
| `RATE_LIMITED` | 429 | Demasiadas peticiones |
| `INTERNAL_ERROR` | 500 | Error interno del servidor |
