# Autenticación

## Mecanismo JWT

El sistema usa **dos tokens JWT**:

| Token | Vida | Almacenamiento |
|-------|------|----------------|
| Access token | 15 minutos | Header `Authorization: Bearer <token>` |
| Refresh token | Configurable (días) | Cookie `httpOnly` + hash en PostgreSQL |

### Estructura del Access Token (Claims)

```json
{
  "sub": 42,
  "uuid": "550e8400-e29b-41d4-a716-446655440000",
  "is_admin": false,
  "exp": 1713456789,
  "iat": 1713455889
}
```

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `sub` | `i64` | ID del usuario |
| `uuid` | `UUID` | UUID del usuario |
| `is_admin` | `bool` | Si el usuario tiene rol admin |
| `exp` | `i64` | Unix timestamp de expiración |
| `iat` | `i64` | Unix timestamp de emisión |

### Algoritmo

- Firma: **HS256** (HMAC-SHA256)
- Hash de refresh tokens: **SHA-256** (almacenado en DB, nunca el token en claro)

---

## Flujo de Autenticación

### Registro y Login

```
POST /v1/auth/register
  → Crea usuario, hashea password con Argon2
  → Devuelve { access_token, user }
  → Setea cookie httpOnly con refresh_token

POST /v1/auth/login
  → Verifica password con Argon2
  → Devuelve { access_token, user }
  → Setea cookie httpOnly con refresh_token
```

### Renovar Token

```
POST /v1/auth/refresh
  → Lee refresh_token de cookie httpOnly
  → Verifica hash en DB (tabla sessions)
  → Devuelve nuevo { access_token }
```

### Logout

```
POST /v1/auth/logout  (requiere Bearer token)
  → Invalida la sesión en DB
  → Limpia la cookie
```

---

## Extractor de Autenticación (Axum)

El crate `shared` provee dos extractores para los handlers de Axum:

### `AuthUser` — Requiere autenticación

```rust
async fn my_handler(auth: AuthUser) -> impl IntoResponse {
    // auth.user_id: i64
    // auth.uuid: Uuid
    // auth.is_admin: bool
}
```

Retorna `401 Unauthorized` si el token falta, es inválido, o está expirado.

### `OptionalAuth` — Autenticación opcional

```rust
async fn my_handler(auth: OptionalAuth) -> impl IntoResponse {
    if let Some(user) = auth.0 {
        // usuario autenticado
    }
}
```

---

## Two-Factor Authentication (2FA)

El sistema soporta 2FA basado en TOTP (compatible con Google Authenticator, Authy, etc.):

1. `POST /v1/auth/2fa/setup` — Genera secreto TOTP y devuelve QR code
2. `POST /v1/auth/2fa/enable` — Activa 2FA verificando el primer código
3. En el login: si 2FA está activo, el login devuelve un token temporal
4. `POST /v1/auth/2fa/verify` — Verifica el código TOTP y devuelve el access token final
5. Códigos de respaldo: `GET /v1/auth/2fa/backup-codes` y `POST .../regenerate`

---

## OAuth Social Login

Endpoint único para todos los proveedores:

```
POST /v1/auth/social/login
Content-Type: application/json

{
  "provider": "google",
  "token": "<provider_access_token>"
}
```

Proveedores soportados:
`google`, `facebook`, `twitter`, `apple`, `linkedin`, `discord`, `tiktok`, `instagram`, `vkontakte`, `qq`, `wechat`, `mailru`, `okru`, `wordpress`

---

## OAuth Developer Portal

Los usuarios pueden crear aplicaciones OAuth para integrar con la plataforma:

```
GET  /v1/oauth/apps              — Listar mis apps
POST /v1/oauth/apps              — Crear app
POST /v1/oauth/authorize         — Autorizar app (flujo OAuth2)
POST /v1/oauth/token             — Intercambiar código por token
POST /v1/oauth/revoke            — Revocar token
```

---

## Gestión de Sesiones

Cada login crea una sesión en la tabla `sessions`:

```
GET    /v1/auth/sessions          — Listar sesiones activas
DELETE /v1/auth/sessions/{id}     — Revocar sesión específica
POST   /v1/auth/sessions/revoke-all — Revocar todas las sesiones
```

---

## Seguridad

- Passwords hasheados con **Argon2id** (resistente a GPU/ASIC)
- Refresh tokens almacenados como **hash SHA-256** (nunca en claro)
- Rate limiting en `/v1/auth/*`: **5 req / 15 min** (protección contra brute force)
- IPs baneadas verificadas en cada request
- Intentos de login fallidos registrados en `login_attempts`
