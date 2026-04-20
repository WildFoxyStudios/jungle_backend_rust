# Background Jobs (jobs-runner)

El `jobs-runner` es un binario independiente que ejecuta 20 tareas en segundo plano de forma periódica. Reemplaza el sistema de cron de la aplicación PHP original.

---

## Lista de Jobs

### 1. `session_cleanup`
**Frecuencia**: Cada hora  
**Descripción**: Elimina sesiones expiradas de la tabla `sessions`. Mantiene la base de datos limpia y reduce el tamaño de la tabla.

---

### 2. `story_cleanup`
**Frecuencia**: Cada hora  
**Descripción**: Elimina stories que han superado las 24 horas de vida. Mueve las stories expiradas al archivo antes de eliminarlas si el usuario tiene archivado activado.

---

### 3. `login_attempts_cleanup`
**Frecuencia**: Cada 6 horas  
**Descripción**: Limpia registros antiguos de intentos de login fallidos de la tabla `login_attempts`. Mantiene solo los últimos 7 días para análisis de seguridad.

---

### 4. `notification_cleanup`
**Frecuencia**: Diaria (medianoche)  
**Descripción**: Elimina notificaciones leídas con más de 30 días de antigüedad. Configurable desde el panel de administración.

---

### 5. `auto_delete_old_messages`
**Frecuencia**: Diaria  
**Descripción**: Elimina mensajes de conversaciones según la política de retención configurada por el admin. Respeta la configuración de auto-eliminación por usuario.

---

### 6. `hashtag_trending`
**Frecuencia**: Cada 15 minutos  
**Descripción**: Recalcula los hashtags trending basándose en el uso en las últimas 24 horas. Actualiza la tabla de trending en Redis para acceso rápido.

---

### 7. `birthday_notifications`
**Frecuencia**: Diaria (8:00 AM)  
**Descripción**: Envía notificaciones a los usuarios cuyos amigos cumplen años hoy. Publica eventos `NotificationCreated` en NATS.

---

### 8. `memories_notification`
**Frecuencia**: Diaria (9:00 AM)  
**Descripción**: Envía notificaciones "En este día" a usuarios que tienen posts de hace exactamente 1, 2, 3... años. Solo envía si el post tiene al menos 1 año.

---

### 9. `weekly_memories_digest`
**Frecuencia**: Semanal (lunes 9:00 AM)  
**Descripción**: Envía un resumen semanal de memorias por email a usuarios que tienen notificaciones de email activadas.

---

### 10. `event_reminders`
**Frecuencia**: Cada 30 minutos  
**Descripción**: Envía recordatorios de eventos próximos a los asistentes confirmados. Envía recordatorio 24 horas antes y 1 hora antes del evento.

---

### 11. `pro_subscription_check`
**Frecuencia**: Diaria  
**Descripción**: Verifica suscripciones Pro expiradas y revoca el estado Pro a usuarios cuya suscripción ha vencido. Envía email de aviso 3 días antes de la expiración.

---

### 12. `ad_budget_check`
**Frecuencia**: Cada hora  
**Descripción**: Verifica el presupuesto de los anuncios activos. Pausa anuncios que han agotado su presupuesto diario o total.

---

### 13. `expire_pending_ads`
**Frecuencia**: Cada hora  
**Descripción**: Expira anuncios que han superado su fecha de fin. Actualiza el estado a `expired` y notifica al anunciante.

---

### 14. `analytics_snapshot_daily`
**Frecuencia**: Diaria (1:00 AM)  
**Descripción**: Toma un snapshot diario de métricas del sitio (usuarios activos, posts creados, mensajes enviados, etc.) para el dashboard de administración.

---

### 15. `publish_scheduled_posts`
**Frecuencia**: Cada minuto  
**Descripción**: Publica posts programados cuya fecha de publicación ha llegado. Cambia el estado de `scheduled` a `published` y publica el evento `PostCreated` en NATS.

---

### 16. `live_stream_cleanup`
**Frecuencia**: Cada 5 minutos  
**Descripción**: Limpia transmisiones en vivo que llevan más de 4 horas activas sin actividad (posibles streams zombies). Marca el stream como terminado.

---

### 17. `crypto_payment_reconciliation`
**Frecuencia**: Cada 10 minutos  
**Descripción**: Verifica el estado de pagos con criptomonedas pendientes (CoinPayments, Coinbase Commerce). Actualiza el estado de las transacciones según la confirmación en blockchain.

---

### 18. `newsletter_dispatcher`
**Frecuencia**: Cada 5 minutos  
**Descripción**: Procesa la cola de newsletters pendientes de envío (tabla `newsletter_queue`). Envía en lotes para respetar los límites del proveedor SMTP.

---

### 19. `dlq_consumer`
**Frecuencia**: Cada 5 minutos  
**Descripción**: Consume mensajes de la Dead Letter Queue de NATS (`dlq.*`). Intenta reprocesar mensajes fallidos. Si fallan de nuevo, los registra en la tabla `dlq_messages` para revisión manual.

---

### 20. `session_cleanup` (duplicado en lista original — ver nota)

> **Nota**: El directorio contiene 20 archivos de jobs. Algunos pueden ser variantes o jobs adicionales no listados en el README original.

---

## Arquitectura del Jobs Runner

```rust
// main.rs del jobs-runner
#[tokio::main]
async fn main() {
    // Inicializa DB, Redis, NATS
    // Lanza todos los jobs como tareas Tokio independientes
    // Cada job tiene su propio intervalo de ejecución
    tokio::join!(
        jobs::session_cleanup::run(state.clone()),
        jobs::story_cleanup::run(state.clone()),
        // ...
    );
}
```

Cada job es una función `async fn run(state: AppState)` que ejecuta un loop infinito con `tokio::time::interval`.

---

## Configuración

Los jobs usan las mismas variables de entorno que los demás servicios:

```bash
DATABASE_URL=...
REDIS_URL=...
NATS_URL=...
```

### Ejecutar el jobs-runner

```bash
# Con Docker
docker compose up -d jobs-runner

# Local
cargo run -p jobs-runner
```
