# Live Service (Self-hosted WebRTC)

Servicio interno para transmisiones en vivo, llamadas de audio y videollamadas sin depender de proveedores externos.

## Objetivo

- Señalización WebRTC propia (`offer`, `answer`, `ice_candidate`, etc.) mediante WebSocket.
- Gestión de salas `live`, `audio_call`, `video_call`.
- Entrega de `iceServers` desde configuración admin (`site_config`) para operar con STUN/TURN propios.

## Endpoints

- `GET /health`
- `POST /v1/live-native/rooms`
- `GET /v1/live-native/rooms`
- `GET /v1/live-native/rooms/{room_id}`
- `POST /v1/live-native/rooms/{room_id}/join`
- `POST /v1/live-native/rooms/{room_id}/leave`
- `GET /v1/live-native/ice-config`
- `GET /ws/live-native?token=<jwt>&room_id=<room_id>`

## Flujo recomendado

1. Crear sala por HTTP.
2. Unir participantes por HTTP.
3. Abrir WebSocket por participante.
4. Intercambiar mensajes de señalización WebRTC.
5. Cerrar/abandonar sala.

## Configuración desde Admin

Se agregan claves en catálogo (categoría `live`):

- `provider=self_hosted_webrtc`
- `native_enabled`
- `native_signaling_url`
- `native_api_base_url`
- `allow_live_streams`
- `allow_audio_calls`
- `allow_video_calls`
- `max_participants_per_room`
- `max_room_duration_minutes`
- `stun_server_url`
- `turn_server_url`
- `turn_username`
- `turn_password`
- `ice_policy`

## Nota operativa

Este servicio resuelve señalización y orquestación. Para NAT restrictivo y mejor conectividad en producción, despliega TURN propio (por ejemplo `coturn`) y registra URL/credenciales en `live`.
