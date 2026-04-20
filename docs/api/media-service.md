# API — Media Service (Puerto 3005)

---

## Subida de Archivos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/media/upload` | Sí | Subir archivo multimedia (multipart) |
| POST | `/v1/media/upload/avatar` | Sí | Subir avatar |
| POST | `/v1/media/upload/cover` | Sí | Subir foto de portada |
| GET | `/v1/media/{id}` | Sí | Obtener info del archivo |
| DELETE | `/v1/media/{id}` | Sí | Eliminar archivo |
| GET | `/v1/media/my` | Sí | Mis archivos subidos |

### POST /v1/media/upload

Request `multipart/form-data`:

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `file` | File | Archivo a subir |
| `type` | string | `image`, `video`, `audio`, `document` |

```json
// Response 201
{
  "data": {
    "id": 123,
    "url": "https://cdn.example.com/media/abc123.jpg",
    "thumbnail_url": "https://cdn.example.com/media/abc123_thumb.jpg",
    "type": "image",
    "size": 204800,
    "width": 1920,
    "height": 1080
  }
}
```

### Límites de Subida

| Tipo | Tamaño máximo |
|------|---------------|
| Imagen | 10 MB |
| Video | 100 MB |
| Audio | 20 MB |
| Documento | 50 MB |

---

## Transformación de Imágenes

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/media/{id}/rotate` | Sí | Rotar imagen |
| POST | `/v1/media/{id}/crop` | Sí | Recortar imagen |

```json
// POST /v1/media/{id}/rotate
{ "degrees": 90 }

// POST /v1/media/{id}/crop
{ "x": 0, "y": 0, "width": 800, "height": 600 }
```

---

## Proveedores de Almacenamiento

Configurado via `STORAGE_PROVIDER`:

| Proveedor | Descripción |
|-----------|-------------|
| `local` | Sistema de archivos local (desarrollo) |
| `s3` | Amazon S3 |
| `minio` | MinIO (S3-compatible, auto-hospedado) |
| `wasabi` | Wasabi Cloud Storage |
| `spaces` | DigitalOcean Spaces |
| `backblaze` | Backblaze B2 |

Los proveedores se pueden configurar dinámicamente desde el panel de administración (`/v1/admin/storage/config`).

---

## Stories

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/stories` | Sí | Feed de stories (de usuarios seguidos) |
| POST | `/v1/stories` | Sí | Crear story |
| GET | `/v1/stories/my` | Sí | Mis stories activas |
| GET | `/v1/stories/archive` | Sí | Stories archivadas (expiradas) |
| GET | `/v1/stories/{id}` | Sí | Obtener story |
| DELETE | `/v1/stories/{id}` | Sí | Eliminar story |
| POST | `/v1/stories/{id}/view` | Sí | Marcar story como vista |
| GET | `/v1/stories/{id}/viewers` | Sí | Listar espectadores |
| POST | `/v1/stories/{id}/react` | Sí | Reaccionar a story |
| GET | `/v1/stories/{id}/reactions` | Sí | Listar reacciones |
| POST | `/v1/stories/{id}/reply` | Sí | Responder a story (envía DM) |

### POST /v1/stories

```json
{
  "media_id": 123,
  "type": "image",
  "duration": 5,
  "privacy": "friends",
  "text_overlay": "¡Buenos días!",
  "background_color": "#FF5733"
}
```

Las stories expiran automáticamente a las 24 horas (gestionado por el job `story_cleanup`). Las stories expiradas se mueven al archivo si el usuario tiene archivado activado.
