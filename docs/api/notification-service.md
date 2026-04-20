# API — Notification Service (Puerto 3006)

---

## Notificaciones

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/notifications` | Sí | Listar notificaciones (cursor pagination) |
| GET | `/v1/notifications/unread-count` | Sí | Obtener conteo de no leídas |
| POST | `/v1/notifications/read-all` | Sí | Marcar todas como leídas |
| POST | `/v1/notifications/{id}/read` | Sí | Marcar notificación como leída |
| DELETE | `/v1/notifications/{id}` | Sí | Eliminar notificación |
| DELETE | `/v1/notifications/clear` | Sí | Limpiar todas las notificaciones |

### GET /v1/notifications

```
GET /v1/notifications?cursor=<cursor>&limit=20
```

```json
// Response
{
  "data": [
    {
      "id": 1,
      "type": "post_like",
      "sender": { "id": 42, "username": "jane", "avatar": "..." },
      "post_id": 100,
      "read": false,
      "created_at": "2026-04-18T10:00:00Z"
    }
  ],
  "meta": { "cursor": "0", "has_more": false }
}
```

### Tipos de Notificación

| Tipo | Descripción |
|------|-------------|
| `post_like` | Alguien reaccionó a tu post |
| `post_comment` | Alguien comentó en tu post |
| `comment_reply` | Alguien respondió a tu comentario |
| `follow` | Alguien te siguió |
| `follow_request` | Solicitud de seguimiento |
| `message` | Nuevo mensaje |
| `group_join` | Alguien se unió a tu grupo |
| `page_like` | Alguien le dio like a tu página |
| `birthday` | Cumpleaños de un amigo |
| `memory` | Memoria "En este día" |
| `event_reminder` | Recordatorio de evento |
| `payment` | Pago completado |
| `mention` | Te mencionaron |

---

## Preferencias

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/notifications/preferences` | Sí | Obtener preferencias |
| PUT | `/v1/notifications/preferences` | Sí | Actualizar preferencias |

```json
// PUT /v1/notifications/preferences
{
  "post_likes": true,
  "comments": true,
  "follows": true,
  "messages": true,
  "birthdays": true,
  "email_notifications": false,
  "push_notifications": true
}
```

---

## Push Tokens

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/notifications/push-tokens` | Sí | Registrar token push (FCM/APNs) |
| GET | `/v1/notifications/push-tokens` | Sí | Listar mis tokens push |
| DELETE | `/v1/notifications/push-tokens/{token}` | Sí | Eliminar token push |

### POST /v1/notifications/push-tokens

```json
{
  "token": "fcm_token_here",
  "platform": "android",
  "device_id": "device_uuid"
}
```

Plataformas: `android` (FCM), `ios` (APNs), `web` (VAPID)

---

## Anuncios del Sistema

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/announcements` | Sí | Listar anuncios activos |
| POST | `/v1/announcements/{id}/dismiss` | Sí | Descartar anuncio |

---

## Newsletter

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/newsletter/subscribe` | No | Suscribirse al newsletter |
| POST | `/v1/newsletter/unsubscribe` | No | Desuscribirse |

```json
// POST /v1/newsletter/subscribe
{ "email": "user@example.com" }
```
