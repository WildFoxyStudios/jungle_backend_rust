# Observabilidad

El sistema implementa los tres pilares de observabilidad: **métricas**, **trazas** y **logs**.

---

## Métricas (Prometheus)

Cada servicio expone un endpoint `/metrics` en formato Prometheus text.

### Endpoint

```
GET http://localhost:{SERVICE_PORT}/metrics
```

### Métricas Disponibles

| Métrica | Tipo | Labels | Descripción |
|---------|------|--------|-------------|
| `Jungle_http_requests_total` | Counter | `method`, `path`, `status` | Total de requests HTTP |
| `Jungle_http_request_duration_seconds` | Histogram | `method`, `path` | Duración de requests |
| `Jungle_db_queries_total` | Counter | `query_type` | Total de queries a DB |
| `Jungle_active_websocket_connections` | Gauge | — | Conexiones WebSocket activas |

### Buckets del Histograma

`0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 5.0` segundos

### Normalización de Paths

Los IDs numéricos y UUIDs en los paths se normalizan a `{id}` para evitar alta cardinalidad:

```
/v1/users/42/posts  →  /v1/users/{id}/posts
/v1/posts/550e8400-e29b-41d4-a716-446655440000  →  /v1/posts/{id}
```

### Configuración de Prometheus

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'jungle-api-gateway'
    static_configs:
      - targets: ['localhost:8080']
  - job_name: 'jungle-auth-service'
    static_configs:
      - targets: ['localhost:3001']
  - job_name: 'jungle-user-service'
    static_configs:
      - targets: ['localhost:3002']
  # ... repetir para cada servicio
```

---

## Trazas (OpenTelemetry)

El sistema exporta trazas distribuidas via **OTLP/gRPC** al colector OpenTelemetry.

### Configuración

```bash
# Variables de entorno para OpenTelemetry
OTEL_EXPORTER_OTLP_ENDPOINT=http://otel-collector:4317
OTEL_SERVICE_NAME=auth-service
RUST_LOG=info,sqlx=warn,tower_http=debug
```

### Integración con Jaeger/Tempo

Las trazas son compatibles con cualquier backend OTLP:
- **Jaeger** — `docker run -p 16686:16686 -p 4317:4317 jaegertracing/all-in-one`
- **Grafana Tempo** — Backend de trazas de Grafana
- **Honeycomb** — SaaS de observabilidad

### Spans Automáticos

El middleware de Axum (`tower-http trace`) genera spans automáticamente para cada request HTTP con:
- `http.method`
- `http.url`
- `http.status_code`
- `http.response_content_length`

---

## Logs (Tracing)

El sistema usa el crate `tracing` con `tracing-subscriber` para logging estructurado.

### Formato

```bash
# Formato texto (desarrollo)
RUST_LOG=info cargo run -p auth-service

# Formato JSON (producción)
RUST_LOG=info,sqlx=warn
```

Ejemplo de log JSON:
```json
{
  "timestamp": "2026-04-18T10:00:00.000Z",
  "level": "INFO",
  "target": "auth_service::handlers::auth",
  "message": "User logged in",
  "user_id": 42,
  "ip": "192.168.1.1"
}
```

### Niveles de Log

| Nivel | Uso |
|-------|-----|
| `ERROR` | Errores que requieren atención inmediata |
| `WARN` | Situaciones anómalas (retry, circuit breaker) |
| `INFO` | Eventos de negocio importantes (login, pago) |
| `DEBUG` | Información de depuración |
| `TRACE` | Información muy detallada (queries SQL) |

### Filtros Recomendados

```bash
# Producción
RUST_LOG=info,sqlx=warn,tower_http=warn

# Desarrollo
RUST_LOG=debug,sqlx=info,tower_http=debug

# Debug de queries SQL
RUST_LOG=info,sqlx=debug
```

---

## Dashboard Grafana

Stack de observabilidad recomendado:

```yaml
# docker-compose.observability.yml
services:
  prometheus:
    image: prom/prometheus
    ports: ["9090:9090"]
    
  grafana:
    image: grafana/grafana
    ports: ["3000:3000"]
    
  jaeger:
    image: jaegertracing/all-in-one
    ports: ["16686:16686", "4317:4317"]
    
  loki:
    image: grafana/loki
    ports: ["3100:3100"]
```

### Dashboards Sugeridos

1. **HTTP Overview** — Requests/s, latencia p50/p95/p99, error rate por servicio
2. **Database** — Queries/s, latencia de queries, conexiones activas
3. **WebSocket** — Conexiones activas, mensajes/s
4. **Business Metrics** — Registros/día, posts/hora, pagos/día

---

## Health Checks

Cada servicio expone `GET /health`:

```json
{
  "status": "ok",
  "service": "auth-service",
  "version": "0.1.0",
  "db": "ok",
  "redis": "ok"
}
```

El API Gateway agrega el health de todos los servicios en `GET /health`.
