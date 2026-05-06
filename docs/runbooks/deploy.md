# Runbook de Despliegue (Backend)

## Objetivo

Publicar una versión backend de forma controlada, verificable y reversible.

## Pre-checks (obligatorios)

1. CI en verde (`cargo check`, tests y build).
2. Cambios de schema revisados.
3. Variables de entorno actualizadas.
4. Ventana de despliegue definida.

## Secuencia de despliegue

1. Desplegar servicios internos afectados.
2. Ejecutar migraciones si aplica.
3. Desplegar `api-gateway`.
4. Correr smoke tests.

## Smoke tests mínimos

- `GET /health` del gateway.
- Login/logout.
- Feed carga.
- Envío de mensaje.
- Subida de media.
- Operación wallet básica (sandbox).
- Acceso admin básico.

## Criterio de éxito

- 5xx sin incremento anómalo.
- latencia p95 estable.
- sin errores críticos en logs.
- métricas de infraestructura normales.

## Rollback

1. Identificar app/servicio causante.
2. Restaurar release anterior del servicio.
3. Si hubo migración incompatible: aplicar plan de reversión de DB.
4. Repetir smoke tests.

## Post-deploy

- Documentar incidente/cambio.
- Registrar versión desplegada y hash.
- Actualizar backlog técnico si se detectaron regresiones.

