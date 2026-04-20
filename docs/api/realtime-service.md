# API — Realtime Service (Puerto 3012)

---

## WebSocket

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/ws` | Sí (query param) | Conexión WebSocket |

### Conexión

```
ws://localhost:8080/ws?token=<access_token>
```

El token JWT se pasa como query parameter. La conexión se rechaza si el token es inválido o ha expirado.

---

## Presencia

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/presence/online` | Sí | Listar usuarios en línea |
| GET | `/v1/presence/{user_id}` | Sí | Verificar si un usuario está en línea |

```json
// GET /v1/presence/42
{
  "data": {
    "user_id": 42,
    "online": true,
    "last_seen": "2026-04-18T10:00:00Z"
  }
}
```

Un usuario se considera "en línea" si tiene una conexión WebSocket activa en el `ConnectionHub`.

---

## Eventos WebSocket (Servidor → Cliente)

El servidor envía mensajes JSON con la estructura:

```json
{
  "event": "new_message",
  "data": { ... }
}
```

### Mensajería

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `new_message` | `{conversation_id, sender_id}` | Nuevo mensaje recibido |
| `message_read` | `{conversation_id, user_id}` | Mensaje leído |
| `typing_start` | `{conversation_id, user_id}` | Usuario empezó a escribir |
| `typing_stop` | `{conversation_id, user_id}` | Usuario dejó de escribir |

### Notificaciones

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `notification` | `{type, sender_id}` | Nueva notificación |

### Presencia

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `presence_online` | `{user_id}` | Usuario se conectó |
| `presence_offline` | `{user_id}` | Usuario se desconectó |

### Llamadas (WebRTC Signaling)

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `incoming_call` | `{call_id, caller_id, call_type}` | Llamada entrante |
| `call_answered` | `{call_id}` | Llamada contestada |
| `call_ended` | `{call_id}` | Llamada terminada |

### Live Streaming

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `live_started` | `{stream_id, user_id}` | Transmisión iniciada |
| `live_ended` | `{stream_id, user_id}` | Transmisión terminada |

### Admin

| Evento | Payload | Descripción |
|--------|---------|-------------|
| `admin_notice` | `{text}` | Aviso del administrador (broadcast a todos) |

---

## Arquitectura Interna

### ConnectionHub

El `ConnectionHub` es el núcleo del servicio. Usa `DashMap` (concurrent HashMap) para gestionar conexiones:

```
user_id → broadcast::Sender<WsMessage>
```

Cada usuario tiene un canal broadcast de capacidad 256 mensajes. Si el canal está lleno, los mensajes más antiguos se descartan.

**Operaciones del hub:**

| Método | Descripción |
|--------|-------------|
| `subscribe(user_id)` | Registra usuario, devuelve receiver |
| `unsubscribe(user_id)` | Elimina conexión del usuario |
| `send_to_user(user_id, msg)` | Envía a un usuario específico |
| `send_to_users(ids, msg)` | Envía a múltiples usuarios |
| `broadcast(msg)` | Envía a todos los conectados |
| `is_online(user_id)` | Verifica si el usuario está conectado |
| `online_users()` | Lista todos los IDs conectados |
| `online_count()` | Número de usuarios conectados |

### Event Consumer

El `event_consumer` se suscribe a `events.>` en NATS y traduce eventos de dominio a mensajes WebSocket:

```
NATS events.> → event_consumer → ConnectionHub → WebSocket clients
```

**Mapeo de eventos NATS → WebSocket:**

| Evento NATS | Evento WS | Destinatario |
|-------------|-----------|--------------|
| `MessageSent` | `new_message` | `recipient_ids` |
| `MessageRead` | `message_read` | `user_id` |
| `TypingStarted` | `typing_start` | `user_id` |
| `TypingStopped` | `typing_stop` | `user_id` |
| `NotificationCreated` | `notification` | `recipient_id` |
| `CallStarted` | `incoming_call` | `callee_id` |
| `LiveStreamStarted` | `live_started` | `user_id` |
| `LiveStreamEnded` | `live_ended` | `user_id` |
| `AdminNotice` | `admin_notice` | Broadcast todos |

Los siguientes eventos NATS **no generan mensajes WebSocket** (son procesados por otros servicios):
`UserCreated`, `UserUpdated`, `UserDeleted`, `FollowCreated`, `FollowDeleted`, `UserBlocked`, `PostCreated`, `PostDeleted`, `PostLiked`, `CommentCreated`, `GroupJoined`, `GroupLeft`, `PageLiked`, `StoryCreated`, `PaymentCompleted`, `NewsletterQueued`

### Typing Indicator

El indicador de escritura usa Redis con TTL de 3 segundos:

```
Key: typing:{conversation_id}:{user_id}
Value: "1"
TTL: 3 segundos
```

Si el usuario deja de escribir, el TTL expira automáticamente y el cliente recibe `typing_stop` via NATS.

---

## Métricas

El gauge `Jungle_active_websocket_connections` en Prometheus refleja el número de conexiones activas en el `ConnectionHub`.
