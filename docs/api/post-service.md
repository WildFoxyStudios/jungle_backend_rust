# API — Post Service (Puerto 3003)

---

## Feed

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/feed` | Sí | Feed personalizado (cursor pagination) |
| GET | `/v1/feed/explore` | Sí | Feed de exploración (posts trending/públicos) |
| GET | `/v1/memories` | Sí | Memorias "En este día" |

### GET /v1/feed

```
GET /v1/feed?cursor=<cursor>&limit=20&filter=all
```

Filtros disponibles: `all`, `photos`, `videos`, `links`, `polls`, `live`

**Algoritmo de ranking**: posts fijados → posts impulsados → score de engagement temporal.  
**Cache**: IDs del feed cacheados en Redis 60 segundos.  
**Anuncios**: se inyecta un anuncio cada 5 posts automáticamente.

```json
// Response
{
  "data": [
    {
      "id": 123,
      "uuid": "...",
      "user_id": 42,
      "content": "Hello world!",
      "post_type": "text",
      "media": [],
      "privacy": "everyone",
      "feeling": "happy",
      "location": "Madrid",
      "is_pinned": false,
      "is_boosted": false,
      "like_count": 15,
      "comment_count": 3,
      "share_count": 1,
      "view_count": 200,
      "publisher": {
        "uuid": "...",
        "username": "john_doe",
        "first_name": "John",
        "last_name": "Doe",
        "avatar": "...",
        "is_verified": true,
        "is_pro": 0
      },
      "created_at": "2026-04-18T10:00:00Z"
    }
  ],
  "meta": { "cursor": "122", "has_more": true }
}
```

---

## Posts CRUD

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/posts` | Sí | Crear post |
| GET | `/v1/posts/{id}` | Sí | Obtener post por ID |
| PUT | `/v1/posts/{id}` | Sí | Actualizar post |
| DELETE | `/v1/posts/{id}` | Sí | Eliminar post (soft delete) |
| GET | `/v1/posts/saved` | Sí | Listar posts guardados |
| POST | `/v1/posts/{id}/save` | Sí | Guardar post |
| DELETE | `/v1/posts/{id}/save` | Sí | Dejar de guardar |
| POST | `/v1/posts/{id}/hide` | Sí | Ocultar post del feed |
| POST | `/v1/posts/{id}/share` | Sí | Compartir/repostear |
| POST | `/v1/posts/{id}/pin` | Sí | Fijar post en perfil |
| DELETE | `/v1/posts/{id}/pin` | Sí | Desfijar post |
| POST | `/v1/posts/{id}/boost` | Sí | Impulsar post (Pro) |
| POST | `/v1/posts/{id}/report` | Sí | Reportar post |
| POST | `/v1/posts/{id}/poll/vote` | Sí | Votar en encuesta |
| GET | `/v1/posts/colored-templates` | No | Listar fondos de color |
| GET | `/v1/posts/reaction-types` | No | Listar tipos de reacción |
| GET | `/v1/posts/most-liked` | Sí | Posts más gustados |
| GET | `/v1/posts/most-watched` | Sí | Posts más vistos |
| GET | `/v1/boosted/posts` | Sí | Mis posts impulsados |

### POST /v1/posts

```json
// Request
{
  "content": "Mi primer post!",
  "privacy": "everyone",
  "media": [{"type": "image", "url": "...", "thumbnail": "..."}],
  "feeling": "happy",
  "location": "Madrid, Spain",
  "colored_post": {"background": "#FF5733", "font_color": "#FFFFFF"},
  "page_id": null,
  "group_id": null,
  "event_id": null,
  "is_reel": false
}
```

El `post_type` se determina automáticamente:
- `is_reel = true` → `"reel"`
- `media` no vacío → `"media"`
- Solo texto → `"text"`

Límite de contenido: 63,206 caracteres.

### GET /v1/posts/{id}

Incrementa `view_count` automáticamente. Incluye info del autor (`publisher`).

### Endpoints XHR adicionales

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/posts/preview-url` | Sí | Preview de URL (Open Graph) |
| POST | `/v1/posts/audio` | Sí | Crear post de audio |
| GET | `/v1/posts/open-to-work` | Sí | Feed de posts "open to work" |
| DELETE | `/v1/posts/{post_id}/media/{media_id}` | Sí | Eliminar media de un post |
| PUT | `/v1/posts/{id}/comments-status` | Sí | Activar/desactivar comentarios |
| POST | `/v1/posts/{id}/mark-sold` | Sí | Marcar producto como vendido |
| POST | `/v1/posts/{id}/notify-followers` | Sí | Notificar a seguidores |
| POST | `/v1/posts/{id}/video-view` | Sí | Registrar vista de video |
| POST | `/v1/posts/{id}/wonder` | Sí | Toggle "wonder" (reacción especial) |
| GET | `/v1/posts/{id}/reactors` | Sí | Listar usuarios que reaccionaron |

---

## Reacciones

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/posts/{id}/react` | Sí | Reaccionar a post |
| DELETE | `/v1/posts/{id}/react` | Sí | Quitar reacción |
| POST | `/v1/comments/{id}/react` | Sí | Reaccionar a comentario |

```json
// POST /v1/posts/{id}/react
{ "reaction_type": "like" }
```

Tipos válidos: `like`, `love`, `haha`, `wow`, `sad`, `angry`

Las reacciones usan `ON CONFLICT DO UPDATE` — si ya existe una reacción del usuario, se actualiza el tipo. El `like_count` se recalcula con un subquery para precisión.

---

## Comentarios

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/posts/{id}/comments` | Sí | Obtener comentarios (paginados, ASC) |
| POST | `/v1/posts/{id}/comments` | Sí | Crear comentario |
| PUT | `/v1/comments/{id}` | Sí | Actualizar comentario |
| DELETE | `/v1/comments/{id}` | Sí | Eliminar comentario |
| GET | `/v1/comments/{id}/replies` | Sí | Obtener respuestas (paginadas, ASC) |
| POST | `/v1/comments/{id}/replies` | Sí | Crear respuesta |

```json
// POST /v1/posts/{id}/comments
{
  "content": "Gran post!",
  "media": [],
  "parent_id": null
}
```

Los comentarios se ordenan **ASC** (más antiguos primero). Las respuestas también.

Al crear un comentario:
- Se incrementa `posts.comment_count`
- Si es respuesta, se incrementa `comments.reply_count` del padre
- Se publica `DomainEvent::CommentCreated` en NATS

Al eliminar:
- Se decrementa el contador con `GREATEST(count - 1, 0)`

---

## Reels

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/reels` | Sí | Feed de reels |
| POST | `/v1/reels` | Sí | Crear reel |
| GET | `/v1/reels/{id}` | Sí | Obtener reel |
| DELETE | `/v1/reels/{id}` | Sí | Eliminar reel |
| POST | `/v1/reels/{id}/view` | Sí | Registrar visualización |
| POST | `/v1/reels/{id}/react` | Sí | Reaccionar a reel |
| GET | `/v1/reels/{id}/comments` | Sí | Comentarios del reel |
| POST | `/v1/reels/{id}/comments` | Sí | Agregar comentario al reel |

Los reels son posts con `is_reel = true`. Se crean via `POST /v1/posts` con `"is_reel": true` o directamente via `POST /v1/reels`.

---

## Búsqueda

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/search` | Sí | Búsqueda global |
| GET | `/v1/search/recent` | Sí | Búsquedas recientes |
| POST | `/v1/search/recent` | Sí | Guardar búsqueda reciente |
| DELETE | `/v1/search/recent` | Sí | Limpiar búsquedas recientes |

```
GET /v1/search?q=javascript&type=user&cursor=<cursor>&limit=20
```

Tipos: `user`, `post`, `page`, `group`, `hashtag`, `blog`, `product`

---

## Hashtags

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/hashtags/trending` | Sí | Hashtags trending (actualizado cada 15 min por job) |
| GET | `/v1/hashtags/search` | Sí | Buscar hashtags (`?q=`) |
| GET | `/v1/hashtags/{tag}/posts` | Sí | Posts por hashtag |

---

## Álbumes

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/albums` | Sí | Crear álbum |
| GET | `/v1/users/{user_id}/albums` | Sí | Listar álbumes del usuario |
| GET | `/v1/albums/{id}/images` | Sí | Listar imágenes del álbum |
| POST | `/v1/albums/{id}/images` | Sí | Agregar imágenes al álbum |
| DELETE | `/v1/albums/{album_id}/images/{image_id}` | Sí | Eliminar imagen del álbum |

---

## Anuncios de Usuario

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/ads` | Sí | Crear anuncio |
| GET | `/v1/ads/my` | Sí | Mis anuncios |
| GET | `/v1/ads/{id}/stats` | Sí | Estadísticas del anuncio |
| PUT | `/v1/ads/{id}` | Sí | Actualizar anuncio |
| DELETE | `/v1/ads/{id}` | Sí | Cancelar anuncio |
| POST | `/v1/ads/{id}/click` | No | Registrar clic |
| POST | `/v1/ads/{id}/view` | No | Registrar visualización |
| GET | `/v1/ads/estimated-audience` | Sí | Audiencia estimada |

---

## Live Streaming

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/live/start` | Sí | Iniciar transmisión en vivo |
| POST | `/v1/live/stop` | Sí | Detener transmisión |
| GET | `/v1/live/active` | Sí | Transmisiones activas |
| GET | `/v1/live/friends` | Sí | Amigos en vivo ahora |
| POST | `/v1/live/{id}/comment` | Sí | Comentar en transmisión |
| POST | `/v1/live/{id}/react` | Sí | Reaccionar a transmisión |
