# API — Auth Service (Puerto 3001)

Base URL: `http://localhost:8080`  
Autenticación: `Authorization: Bearer <access_token>` (excepto rutas públicas)

---

## Autenticación Core

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/auth/register` | No | Registrar nuevo usuario |
| POST | `/v1/auth/login` | No | Login con email/teléfono + contraseña |
| POST | `/v1/auth/refresh` | No | Renovar access token (usa cookie httpOnly) |
| POST | `/v1/auth/logout` | Sí | Cerrar sesión actual |
| GET | `/v1/auth/me` | Sí | Obtener usuario autenticado actual |

### POST /v1/auth/register

```json
// Request
{
  "username": "john_doe",
  "email": "john@example.com",
  "password": "secret123",
  "first_name": "John",
  "last_name": "Doe"
}

// Response 201
{
  "data": {
    "access_token": "eyJ...",
    "user": { "id": 1, "username": "john_doe", ... }
  }
}
```

### POST /v1/auth/login

```json
// Request
{
  "email": "john@example.com",
  "password": "secret123"
}

// Response 200
{
  "data": {
    "access_token": "eyJ...",
    "user": { ... },
    "requires_2fa": false
  }
}
```

Si `requires_2fa: true`, el cliente debe llamar a `/v1/auth/2fa/verify` con el código TOTP.

---

## Contraseña

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/auth/forgot-password` | No | Enviar email de recuperación |
| POST | `/v1/auth/reset-password` | No | Resetear contraseña con token |
| PUT | `/v1/auth/password` | Sí | Cambiar contraseña |

### POST /v1/auth/forgot-password

```json
{ "email": "john@example.com" }
```

### POST /v1/auth/reset-password

```json
{
  "token": "<reset_token_from_email>",
  "password": "new_password123"
}
```

### PUT /v1/auth/password

```json
{
  "current_password": "old_password",
  "new_password": "new_password123"
}
```

---

## Verificación

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/auth/verify-email` | No | Verificar email con código |
| POST | `/v1/auth/verify-phone` | No | Verificar teléfono con código |
| POST | `/v1/auth/resend-code` | No | Reenviar código de verificación |

---

## Two-Factor Auth (2FA)

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/auth/2fa/setup` | Sí | Configurar 2FA (devuelve QR/secreto) |
| POST | `/v1/auth/2fa/enable` | Sí | Activar 2FA tras configuración |
| POST | `/v1/auth/2fa/verify` | No | Verificar código 2FA durante login |
| POST | `/v1/auth/2fa/disable` | Sí | Desactivar 2FA |
| GET | `/v1/auth/2fa/backup-codes` | Sí | Listar códigos de respaldo |
| POST | `/v1/auth/2fa/backup-codes/regenerate` | Sí | Regenerar códigos de respaldo |

---

## Login Social

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/auth/social/login` | No | Login social (body: `{provider, token}`) |

Proveedores: `google`, `facebook`, `twitter`, `apple`, `linkedin`, `discord`, `tiktok`, `instagram`, `vkontakte`, `qq`, `wechat`, `mailru`, `okru`, `wordpress`

```json
// Request
{
  "provider": "google",
  "token": "<google_access_token>"
}
```

---

## Sesiones

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/auth/sessions` | Sí | Listar sesiones activas |
| DELETE | `/v1/auth/sessions/{id}` | Sí | Revocar sesión específica |
| POST | `/v1/auth/sessions/revoke-all` | Sí | Revocar todas las sesiones |
| POST | `/v1/auth/switch-account` | Sí | Cambiar a otra cuenta |

---

## OAuth Developer Portal

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/oauth/apps` | Sí | Listar mis apps OAuth |
| POST | `/v1/oauth/apps` | Sí | Crear app OAuth |
| GET | `/v1/oauth/apps/{id}` | Sí | Obtener detalles de app |
| PUT | `/v1/oauth/apps/{id}` | Sí | Actualizar app |
| DELETE | `/v1/oauth/apps/{id}` | Sí | Eliminar app |
| GET | `/v1/oauth/apps/{id}/permissions` | Sí | Permisos de la app |
| POST | `/v1/oauth/authorize` | Sí | Autorizar app OAuth |
| POST | `/v1/oauth/token` | No | Intercambiar código por token |
| POST | `/v1/oauth/revoke` | Sí | Revocar token OAuth |

---

## Rutas Públicas

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/translations/{lang}` | No | Obtener traducciones para un idioma |
| GET | `/v1/config/public` | No | Configuración pública del sitio |
| GET | `/v1/site-settings` | No | Configuración del sitio |
| GET | `/v1/auth/check` | No | Verificar disponibilidad de username/email |
| GET | `/v1/auth/is-active` | No | Verificar si el sitio está activo |

### GET /v1/auth/check

```
GET /v1/auth/check?username=john_doe
GET /v1/auth/check?email=john@example.com
```

```json
// Response
{ "data": { "available": true } }
```
