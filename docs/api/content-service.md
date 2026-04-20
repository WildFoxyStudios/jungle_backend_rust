# API — Content Service (Puerto 3008)

---

## Blogs

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/blogs` | Sí | Listar blogs |
| POST | `/v1/blogs` | Sí | Crear blog |
| GET | `/v1/blogs/search` | Sí | Buscar blogs |
| GET | `/v1/blogs/my` | Sí | Mis blogs |
| GET | `/v1/blogs/categories` | No | Listar categorías de blogs |
| GET | `/v1/blogs/category/{id}` | Sí | Blogs por categoría |
| GET | `/v1/blogs/{id}` | Sí | Obtener blog |
| PUT | `/v1/blogs/{id}` | Sí | Actualizar blog |
| DELETE | `/v1/blogs/{id}` | Sí | Eliminar blog |
| GET | `/v1/blogs/{id}/comments` | Sí | Listar comentarios del blog |
| POST | `/v1/blogs/{id}/comments` | Sí | Agregar comentario |
| DELETE | `/v1/blogs/comments/{id}` | Sí | Eliminar comentario |
| POST | `/v1/blogs/upload-image` | Sí | Subir imagen para el editor |
| POST | `/v1/blogs/{id}/react` | Sí | Reaccionar al blog |
| POST | `/v1/blogs/comments/{id}/react` | Sí | Reaccionar a comentario |

### POST /v1/blogs

```json
{
  "title": "Introducción a Rust",
  "content": "<p>Rust es un lenguaje...</p>",
  "category_id": 2,
  "thumbnail_id": 45,
  "tags": ["rust", "programación"],
  "status": "published"
}
```

Estados: `draft`, `published`

---

## Foros

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/forums/sections` | Sí | Listar secciones del foro |
| GET | `/v1/forums/{id}/threads` | Sí | Listar hilos en un foro |
| POST | `/v1/forums/{id}/threads` | Sí | Crear hilo |
| GET | `/v1/forums/threads/{id}` | Sí | Obtener hilo |
| PUT | `/v1/forums/threads/{id}` | Sí | Actualizar hilo |
| DELETE | `/v1/forums/threads/{id}` | Sí | Eliminar hilo |
| GET | `/v1/forums/threads/{id}/replies` | Sí | Listar respuestas |
| POST | `/v1/forums/threads/{id}/replies` | Sí | Crear respuesta |
| PUT | `/v1/forums/replies/{id}` | Sí | Actualizar respuesta |
| DELETE | `/v1/forums/replies/{id}` | Sí | Eliminar respuesta |
| POST | `/v1/forums/threads/{id}/vote` | Sí | Votar en hilo |
| POST | `/v1/forums/threads/{id}/share` | Sí | Compartir hilo |

---

## Películas

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/movies` | Sí | Listar películas |
| POST | `/v1/movies` | Sí | Crear película |
| GET | `/v1/movies/{id}` | Sí | Obtener película |
| PUT | `/v1/movies/{id}` | Sí | Actualizar película |
| DELETE | `/v1/movies/{id}` | Sí | Eliminar película |
| GET | `/v1/movies/{id}/comments` | Sí | Comentarios de la película |
| POST | `/v1/movies/{id}/comments` | Sí | Agregar comentario |
| POST | `/v1/movies/{id}/react` | Sí | Reaccionar a película |
| POST | `/v1/movies/{id}/watch` | Sí | Registrar visualización |

### POST /v1/movies

```json
{
  "title": "Mi Película",
  "description": "Descripción...",
  "video_url": "https://...",
  "thumbnail_id": 10,
  "category_id": 3,
  "year": 2026,
  "duration": 120
}
```

---

## Juegos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/games` | No | Listar juegos |
| GET | `/v1/games/my` | Sí | Mis juegos jugados recientemente |
| GET | `/v1/games/{id}` | No | Obtener juego |
| POST | `/v1/games/{id}/play` | Sí | Registrar partida |

---

## Páginas Personalizadas (Públicas)

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/pages/custom` | No | Listar páginas personalizadas |
| GET | `/v1/pages/custom/{slug}` | No | Obtener página por slug |

Estas son páginas estáticas de contenido creadas desde el panel de administración (ej: "Términos de uso", "Política de privacidad").
