# Event Bus (NATS)

El sistema usa **NATS** como bus de eventos asíncrono para comunicación entre microservicios. Los eventos fluyen principalmente desde los servicios de negocio hacia `notification-service` y `realtime-service`.

---

## Arquitectura

```
auth-service ──────────────────────────────────────────────────────┐
user-service ──────────────────────────────────────────────────────┤
post-service ──────────────────────────────────────────────────────┤
messaging-service ─────────────────────────────────────────────────┤
                                                                    ▼
                                                          ┌──────────────────┐
                                                          │   NATS Server    │
                                                          │   :4222          │
                                                          └──────────────────┘
                                                                    │
                                              ┌─────────────────────┼──────────────────┐
                                              ▼                     ▼                  ▼
                                   notification-service    realtime-service      jobs-runner
```

---

## Eventos de Dominio

### Usuarios

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `UserCreated` | `events.user.created` | `{user_id, username}` |
| `UserUpdated` | `events.user.updated` | `{user_id, fields: [String]}` |
| `UserDeleted` | `events.user.deleted` | `{user_id}` |

### Grafo Social

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `FollowCreated` | `events.follow.created` | `{follower_id, following_id}` |
| `FollowDeleted` | `events.follow.deleted` | `{follower_id, following_id}` |
| `UserBlocked` | `events.user.blocked` | `{blocker_id, blocked_id}` |

### Posts

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `PostCreated` | `events.post.created` | `{post_id, user_id, group_id?, page_id?}` |
| `PostDeleted` | `events.post.deleted` | `{post_id}` |
| `PostLiked` | `events.post.liked` | `{post_id, user_id, author_id, reaction_type}` |
| `CommentCreated` | `events.post.commented` | `{comment_id, post_id, user_id, author_id}` |

### Mensajería

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `MessageSent` | `events.message.sent` | `{conversation_id, sender_id, recipient_ids}` |
| `MessageRead` | `events.message.read` | `{conversation_id, user_id}` |
| `TypingStarted` | `events.typing.start` | `{conversation_id, user_id}` |
| `TypingStopped` | `events.typing.stop` | `{conversation_id, user_id}` |

### Grupos y Páginas

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `GroupJoined` | `events.group.joined` | `{group_id, user_id}` |
| `GroupLeft` | `events.group.left` | `{group_id, user_id}` |
| `PageLiked` | `events.page.liked` | `{page_id, user_id}` |

### Stories

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `StoryCreated` | `events.story.created` | `{story_id, user_id}` |

### Llamadas

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `CallStarted` | `events.call.started` | `{call_id, caller_id, callee_id, call_type}` |
| `CallAnswered` | `events.call.answered` | `{call_id}` |
| `CallEnded` | `events.call.ended` | `{call_id}` |

### Pagos

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `PaymentCompleted` | `events.payment.completed` | `{transaction_id, user_id, amount, tx_type}` |

### Live Streaming

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `LiveStreamStarted` | `events.live.started` | `{stream_id, user_id}` |
| `LiveStreamEnded` | `events.live.ended` | `{stream_id, user_id}` |

### Notificaciones

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `NotificationCreated` | `events.notification.created` | `{recipient_id, notification_type, sender_id?}` |

### Presencia

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `PresenceOnline` | `events.presence.online` | `{user_id}` |
| `PresenceOffline` | `events.presence.offline` | `{user_id}` |

### Admin

| Evento | Sujeto NATS | Payload |
|--------|-------------|---------|
| `AdminNotice` | `events.admin.notice` | `{text, target}` |
| `NewsletterQueued` | `events.admin.newsletter` | `{subject, recipient_count}` |

---

## Formato de Serialización

Los eventos se serializan como JSON con la estructura:

```json
{
  "event": "PostCreated",
  "data": {
    "post_id": 123,
    "user_id": 42,
    "group_id": null,
    "page_id": null
  }
}
```

---

## Publicación con Retry

El `NatsEventBus` implementa retry automático con backoff exponencial:

1. Intento 1 — inmediato
2. Intento 2 — espera 100ms
3. Intento 3 — espera 200ms
4. Si falla el intento 3 → mensaje enviado a `dlq.<subject>` (Dead Letter Queue)

---

## Dead Letter Queue (DLQ)

Los mensajes que fallan tras 3 intentos se envían al sujeto `dlq.<original_subject>`.

El job `dlq_consumer` (en `jobs-runner`) procesa periódicamente la DLQ e intenta reprocesar los mensajes fallidos.

---

## Uso en Código

### Publicar un evento

```rust
use shared::events::{DomainEvent, EventBus};

// En un handler
state.event_bus.publish(&DomainEvent::PostCreated {
    post_id: new_post.id,
    user_id: auth.user_id,
    group_id: None,
    page_id: None,
}).await.ok(); // Los errores de publicación no deben fallar el request
```

### Suscribirse a eventos

```rust
let mut sub = event_bus.subscribe("events.post.*").await?;
while let Some((subject, event)) = sub.next().await {
    match event {
        DomainEvent::PostCreated { post_id, user_id, .. } => {
            // procesar evento
        }
        _ => {}
    }
}
```

---

## Configuración NATS

NATS corre con JetStream habilitado para persistencia de mensajes:

```yaml
# docker-compose.yml
nats:
  command: ["--js", "--sd", "/data", "--http_port", "8222"]
```

El panel de monitoreo NATS está disponible en `http://localhost:8222`.
