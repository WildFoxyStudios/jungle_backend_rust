# API — Group & Page Service (Puerto 3007)

---

## Páginas

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/pages` | Sí | Crear página |
| GET | `/v1/pages/categories` | No | Listar categorías de páginas |
| GET | `/v1/pages/search` | Sí | Buscar páginas |
| GET | `/v1/pages/suggested` | Sí | Páginas sugeridas |
| GET | `/v1/pages/my` | Sí | Mis páginas |
| GET | `/v1/pages/liked` | Sí | Páginas que me gustan |
| GET | `/v1/pages/check-name` | Sí | Verificar disponibilidad de nombre |
| GET | `/v1/pages/{slug}` | Sí | Obtener página por slug |
| PUT | `/v1/pages/{id}` | Sí | Actualizar página |
| DELETE | `/v1/pages/{id}` | Sí | Eliminar página |
| POST | `/v1/pages/{id}/like` | Sí | Dar like a página |
| DELETE | `/v1/pages/{id}/like` | Sí | Quitar like |
| POST | `/v1/pages/{id}/rate` | Sí | Calificar página (1-5) |
| GET | `/v1/pages/{id}/likes` | Sí | Listar usuarios que dieron like |
| GET | `/v1/pages/{id}/ratings` | Sí | Listar calificaciones |
| GET | `/v1/pages/{id}/admins` | Sí | Listar admins de la página |
| POST | `/v1/pages/{id}/admins` | Sí | Agregar admin |
| DELETE | `/v1/pages/{id}/admins/{user_id}` | Sí | Eliminar admin |
| GET | `/v1/pages/{id}/posts` | Sí | Posts de la página |
| POST | `/v1/pages/{id}/invite` | Sí | Invitar usuarios a dar like |
| PUT | `/v1/pages/{id}/avatar` | Sí | Actualizar avatar de página |
| PUT | `/v1/pages/{id}/cover` | Sí | Actualizar portada |
| POST | `/v1/pages/{id}/boost` | Sí | Impulsar página (Pro) |
| POST | `/v1/pages/{id}/verify` | Sí | Solicitar verificación |
| GET | `/v1/pages/{id}/non-likes` | Sí | Usuarios que no dieron like |
| GET | `/v1/boosted/pages` | Sí | Mis páginas impulsadas |

### POST /v1/pages

```json
{
  "name": "Mi Empresa",
  "slug": "mi-empresa",
  "category_id": 3,
  "description": "Descripción de la página",
  "website": "https://miempresa.com",
  "phone": "+34 600 000 000",
  "address": "Calle Mayor 1, Madrid"
}
```

---

## Grupos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/groups` | Sí | Crear grupo |
| GET | `/v1/groups/categories` | No | Listar categorías de grupos |
| GET | `/v1/groups/search` | Sí | Buscar grupos |
| GET | `/v1/groups/suggested` | Sí | Grupos sugeridos |
| GET | `/v1/groups/my` | Sí | Mis grupos (creados) |
| GET | `/v1/groups/joined` | Sí | Grupos a los que me uní |
| GET | `/v1/groups/check-name` | Sí | Verificar disponibilidad de nombre |
| GET | `/v1/groups/{slug}` | Sí | Obtener grupo por slug |
| PUT | `/v1/groups/{id}` | Sí | Actualizar grupo |
| DELETE | `/v1/groups/{id}` | Sí | Eliminar grupo |
| POST | `/v1/groups/{id}/join` | Sí | Unirse al grupo |
| DELETE | `/v1/groups/{id}/join` | Sí | Abandonar grupo |
| GET | `/v1/groups/{id}/members` | Sí | Listar miembros |
| DELETE | `/v1/groups/{id}/members/{uid}` | Sí | Expulsar miembro |
| POST | `/v1/groups/{id}/members/{uid}/role` | Sí | Cambiar rol de miembro |
| GET | `/v1/groups/{id}/join-requests` | Sí | Solicitudes de unión |
| POST | `/v1/groups/{id}/join-requests/{rid}/accept` | Sí | Aceptar solicitud |
| POST | `/v1/groups/{id}/join-requests/{rid}/reject` | Sí | Rechazar solicitud |
| GET | `/v1/groups/{id}/posts` | Sí | Posts del grupo |
| POST | `/v1/groups/{id}/invite` | Sí | Invitar usuarios al grupo |
| PUT | `/v1/groups/{id}/avatar` | Sí | Actualizar avatar del grupo |
| PUT | `/v1/groups/{id}/cover` | Sí | Actualizar portada |
| GET | `/v1/groups/{id}/non-members` | Sí | Usuarios que no son miembros |

### POST /v1/groups

```json
{
  "name": "Desarrolladores Rust",
  "slug": "devs-rust",
  "category_id": 5,
  "description": "Comunidad de Rust en español",
  "privacy": "public"
}
```

Privacidad: `public`, `closed`, `secret`

---

## Eventos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/events` | Sí | Crear evento |
| GET | `/v1/events/upcoming` | Sí | Próximos eventos |
| GET | `/v1/events/my` | Sí | Mis eventos |
| GET | `/v1/events/attending` | Sí | Eventos a los que asistiré |
| GET | `/v1/events/{id}` | Sí | Obtener evento |
| PUT | `/v1/events/{id}` | Sí | Actualizar evento |
| DELETE | `/v1/events/{id}` | Sí | Eliminar evento |
| POST | `/v1/events/{id}/respond` | Sí | RSVP al evento |
| GET | `/v1/events/{id}/going` | Sí | Listar asistentes confirmados |
| GET | `/v1/events/{id}/interested` | Sí | Listar interesados |
| POST | `/v1/events/{id}/invite` | Sí | Invitar usuarios |
| GET | `/v1/events/{id}/posts` | Sí | Posts del evento |
| PUT | `/v1/events/{id}/cover` | Sí | Actualizar portada del evento |

### POST /v1/events

```json
{
  "name": "Meetup Rust Madrid",
  "description": "Encuentro mensual de la comunidad Rust",
  "start_date": "2026-05-15T18:00:00Z",
  "end_date": "2026-05-15T21:00:00Z",
  "location": "Madrid, Spain",
  "privacy": "public",
  "category_id": 5
}
```

### POST /v1/events/{id}/respond

```json
{ "response": "going" }
```

Respuestas: `going`, `interested`, `not_going`
