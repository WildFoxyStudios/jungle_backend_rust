# Resiliencia

El crate `shared` provee primitivas de resiliencia para manejar fallos en servicios externos y comunicación inter-servicio.

---

## Circuit Breaker

Implementado en `shared::resilience::CircuitBreaker`.

### Comportamiento

```
Estado CLOSED (normal)
    │
    │ N fallos consecutivos
    ▼
Estado OPEN (rechaza requests)
    │
    │ timeout_secs transcurridos
    ▼
Estado HALF-OPEN (permite 1 request de prueba)
    │
    ├── Éxito → vuelve a CLOSED
    └── Fallo → vuelve a OPEN
```

### Uso

```rust
use shared::resilience::CircuitBreaker;

let cb = CircuitBreaker::new(
    3,   // threshold: abre tras 3 fallos
    30,  // timeout_secs: espera 30s antes de half-open
);

// Ejecutar operación a través del circuit breaker
let result = cb.call(async {
    external_service.call().await
}).await;

match result {
    Ok(val) => { /* éxito, circuit breaker registra success */ }
    Err(ApiError::Internal(msg)) if msg.contains("Circuit breaker open") => {
        // servicio no disponible
    }
    Err(e) => { /* fallo normal, circuit breaker registra failure */ }
}
```

### Estados

| Estado | Descripción |
|--------|-------------|
| `Closed` | Normal. Todas las requests pasan. |
| `Open` | Rechaza todas las requests inmediatamente con error. |
| `HalfOpen` | Permite una request de prueba para verificar recuperación. |

---

## Retry con Backoff Exponencial

### `retry_with_backoff`

```rust
use shared::resilience::retry_with_backoff;

let result = retry_with_backoff(
    || async { external_api.call().await },
    3, // max_retries (total intentos = 4)
).await;
```

**Delays**: 100ms → 200ms → 400ms (base 100ms, duplica cada intento)

### `retry_boxed`

Para closures que no implementan `FnMut` fácilmente:

```rust
use shared::resilience::retry_boxed;

let result = retry_boxed(
    || Box::pin(async { external_api.call().await }),
    3,
).await;
```

---

## Publish con Retry (NATS)

### `publish_with_retry`

```rust
use shared::resilience::publish_with_retry;

publish_with_retry(
    &nats_client,
    "events.post.created",
    &payload_bytes,
).await;
```

- 3 intentos con backoff exponencial
- Si falla el intento 3 → mensaje enviado a `dlq.events.post.created`
- Los errores de publicación se loguean pero no propagan (fire-and-forget)

---

## Dead Letter Queue (DLQ)

Los mensajes NATS que fallan tras 3 intentos se envían al sujeto `dlq.<original_subject>`.

### Procesamiento de DLQ

El job `dlq_consumer` en `jobs-runner` procesa la DLQ cada 5 minutos:

1. Consume mensajes de `dlq.*`
2. Intenta reprocesar cada mensaje
3. Si falla de nuevo → registra en tabla `dlq_messages` para revisión manual
4. Los mensajes procesados exitosamente se eliminan de la DLQ

### Tabla `dlq_messages`

```sql
CREATE TABLE dlq_messages (
    id BIGSERIAL PRIMARY KEY,
    subject TEXT NOT NULL,
    payload JSONB NOT NULL,
    error TEXT,
    attempts INTEGER DEFAULT 0,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    processed_at TIMESTAMPTZ
);
```

---

## Timeouts

Cada servicio configura timeouts en el cliente HTTP inter-servicio:

```rust
// internal_client.rs
reqwest::Client::builder()
    .timeout(Duration::from_secs(10))
    .connect_timeout(Duration::from_secs(3))
    .build()
```

---

## Rate Limiting

El API Gateway implementa rate limiting por IP usando Redis:

| Endpoint | Límite | Ventana |
|----------|--------|---------|
| `/v1/auth/*` | 5 requests | 15 minutos |
| `/v1/media/upload` | 20 requests | 1 minuto |
| `*/search*` | 30 requests | 1 minuto |
| `/v1/messages/*` | 60 requests | 1 minuto |
| Todo lo demás | 100 requests | 1 minuto |

Cuando se supera el límite, el servidor responde con `429 Too Many Requests` y el header `Retry-After`.

---

## Graceful Shutdown

Todos los servicios manejan señales `SIGTERM` y `SIGINT` para shutdown graceful:

1. Dejan de aceptar nuevas conexiones
2. Esperan a que las conexiones activas terminen (timeout: 30s)
3. Cierran el pool de DB y la conexión Redis
4. Terminan el proceso

Esto garantiza que no se pierdan requests en curso durante despliegues.
