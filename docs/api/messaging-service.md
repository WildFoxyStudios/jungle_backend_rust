# API — Messaging Service (Puerto 3004)

---

## Conversaciones

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/conversations` | Sí | Listar conversaciones |
| POST | `/v1/conversations` | Sí | Crear conversación directa |
| POST | `/v1/conversations/group` | Sí | Crear conversación grupal |
| GET | `/v1/conversations/pinned` | Sí | Conversaciones fijadas |
| GET | `/v1/conversations/archived` | Sí | Conversaciones archivadas |
| GET | `/v1/conversations/{id}` | Sí | Obtener conversación |
| DELETE | `/v1/conversations/{id}` | Sí | Eliminar conversación |
| PUT | `/v1/conversations/{id}/color` | Sí | Cambiar color de conversación |
| POST | `/v1/conversations/{id}/pin` | Sí | Fijar conversación |
| DELETE | `/v1/conversations/{id}/pin` | Sí | Desfijar conversación |
| POST | `/v1/conversations/{id}/archive` | Sí | Archivar conversación |
| DELETE | `/v1/conversations/{id}/archive` | Sí | Desarchivar conversación |
| POST | `/v1/conversations/{id}/read` | Sí | Marcar conversación como leída |
| POST | `/v1/conversations/mark-all-read` | Sí | Marcar todas como leídas |
| PUT | `/v1/conversations/group/{id}` | Sí | Actualizar info de grupo |

### POST /v1/conversations

```json
{ "user_id": 42 }
```

### POST /v1/conversations/group

```json
{
  "name": "Equipo Dev",
  "member_ids": [42, 43, 44],
  "avatar_id": 5
}
```

---

## Mensajes

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/conversations/{id}/messages` | Sí | Listar mensajes (cursor pagination) |
| POST | `/v1/conversations/{id}/messages` | Sí | Enviar mensaje |
| POST | `/v1/conversations/{id}/typing` | Sí | Enviar indicador de escritura |
| DELETE | `/v1/messages/{id}` | Sí | Eliminar mensaje |
| POST | `/v1/messages/{id}/favorite` | Sí | Toggle favorito en mensaje |
| POST | `/v1/messages/{id}/pin` | Sí | Fijar mensaje |
| DELETE | `/v1/messages/{id}/pin` | Sí | Desfijar mensaje |
| POST | `/v1/messages/{id}/forward` | Sí | Reenviar mensaje |
| POST | `/v1/messages/{id}/react` | Sí | Reaccionar a mensaje |
| POST | `/v1/messages/{id}/listened` | Sí | Marcar audio como escuchado |

### POST /v1/conversations/{id}/messages

```json
{
  "content": "Hola!",
  "media_ids": [],
  "reply_to_id": null,
  "sticker_id": null
}
```

Tipos de mensaje soportados: texto, imagen, video, audio, sticker, GIF, ubicación, archivo.

---

## Broadcasts

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/broadcasts` | Sí | Listar broadcasts |
| POST | `/v1/broadcasts` | Sí | Crear broadcast |
| PUT | `/v1/broadcasts/{id}` | Sí | Actualizar broadcast |
| DELETE | `/v1/broadcasts/{id}` | Sí | Eliminar broadcast |
| GET | `/v1/broadcasts/{id}/members` | Sí | Listar miembros |
| POST | `/v1/broadcasts/{id}/members` | Sí | Agregar miembros |
| DELETE | `/v1/broadcasts/{id}/members/{user_id}` | Sí | Eliminar miembro |
| POST | `/v1/broadcasts/{id}/send` | Sí | Enviar mensaje broadcast |

---

## Llamadas

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/calls` | Sí | Historial de llamadas |
| POST | `/v1/calls` | Sí | Iniciar llamada |
| POST | `/v1/calls/agora-token` | Sí | Generar token Agora |
| GET | `/v1/calls/{id}` | Sí | Detalles de llamada |
| PUT | `/v1/calls/{id}/status` | Sí | Actualizar estado (accept/reject/end) |

### POST /v1/calls

```json
{
  "callee_id": 42,
  "call_type": "video"
}
```

Tipos: `audio`, `video`

### PUT /v1/calls/{id}/status

```json
{ "status": "accepted" }
```

Estados: `accepted`, `rejected`, `ended`, `missed`
