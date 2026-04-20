# API — User Service (Puerto 3002)

---

## Perfil

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/users/me` | Sí | Obtener mi perfil |
| PUT | `/v1/users/me` | Sí | Actualizar mi perfil |
| DELETE | `/v1/users/me` | Sí | Eliminar mi cuenta |
| PUT | `/v1/users/me/avatar` | Sí | Actualizar avatar |
| PUT | `/v1/users/me/cover` | Sí | Actualizar foto de portada |
| GET | `/v1/users/{username}` | Sí | Obtener perfil por username |
| PUT | `/v1/users/me/social-links` | Sí | Actualizar enlaces sociales |
| GET | `/v1/users/{username}/social-links` | No | Obtener enlaces sociales del usuario |

### PUT /v1/users/me

```json
{
  "first_name": "John",
  "last_name": "Doe",
  "bio": "Developer & coffee lover",
  "website": "https://johndoe.com",
  "location": "Madrid, Spain",
  "birth_date": "1990-01-15"
}
```

---

## Búsqueda y Descubrimiento

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/users/search` | Sí | Buscar usuarios |
| GET | `/v1/users/suggestions` | Sí | Sugerencias de seguimiento |
| GET | `/v1/users/pro-users` | Sí | Listar usuarios Pro |
| GET | `/v1/mentions` | Sí | Autocompletado de menciones (`?q=`) |
| GET | `/v1/users/nearby` | Sí | Usuarios cercanos (requiere ubicación) |
| GET | `/v1/users/birthdays` | Sí | Amigos con cumpleaños hoy |
| POST | `/v1/users/batch` | Sí | Obtener usuarios por IDs (batch) |
| GET | `/v1/users/by-phone` | Sí | Obtener usuario por teléfono |

### GET /v1/users/search

```
GET /v1/users/search?q=john&cursor=<cursor>&limit=20
```

---

## Grafo Social

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/social/follow/{user_id}` | Sí | Seguir usuario |
| DELETE | `/v1/social/follow/{user_id}` | Sí | Dejar de seguir |
| GET | `/v1/users/{username}/followers` | Sí | Listar seguidores |
| GET | `/v1/users/{username}/following` | Sí | Listar seguidos |
| GET | `/v1/social/follow-requests` | Sí | Solicitudes de seguimiento pendientes |
| POST | `/v1/social/follow-requests/{id}/accept` | Sí | Aceptar solicitud |
| POST | `/v1/social/follow-requests/{id}/reject` | Sí | Rechazar solicitud |
| GET | `/v1/social/blocked` | Sí | Listar usuarios bloqueados |
| POST | `/v1/social/block/{user_id}` | Sí | Bloquear usuario |
| DELETE | `/v1/social/block/{user_id}` | Sí | Desbloquear usuario |
| POST | `/v1/social/poke/{user_id}` | Sí | Dar un toque al usuario |
| POST | `/v1/social/mute/{user_id}` | Sí | Silenciar usuario |
| DELETE | `/v1/social/mute/{user_id}` | Sí | Dejar de silenciar |
| POST | `/v1/social/family/{user_id}` | Sí | Enviar solicitud de familia |
| PUT | `/v1/social/family/{id}` | Sí | Responder a solicitud de familia |
| POST | `/v1/social/stop-notify/{user_id}` | Sí | Dejar de recibir notificaciones de posts |

---

## Perfil Profesional (Modo LinkedIn)

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/users/{user_id}/experience` | Sí | Obtener experiencia laboral |
| POST | `/v1/users/me/experience` | Sí | Agregar experiencia laboral |
| DELETE | `/v1/users/me/experience/{id}` | Sí | Eliminar experiencia |
| GET | `/v1/users/{user_id}/certifications` | Sí | Obtener certificaciones |
| POST | `/v1/users/me/certifications` | Sí | Agregar certificación |
| DELETE | `/v1/users/me/certifications/{id}` | Sí | Eliminar certificación |
| GET | `/v1/users/{user_id}/projects` | Sí | Obtener proyectos |
| POST | `/v1/users/me/projects` | Sí | Agregar proyecto |
| DELETE | `/v1/users/me/projects/{id}` | Sí | Eliminar proyecto |
| GET | `/v1/users/{user_id}/mutual-friends` | Sí | Amigos en común |
| GET | `/v1/users/{username}/skills` | Sí | Habilidades del usuario |
| GET | `/v1/skills/search` | Sí | Autocompletado de habilidades |
| POST | `/v1/users/me/skills` | Sí | Agregar habilidad |
| DELETE | `/v1/users/me/skills/{id}` | Sí | Eliminar habilidad |
| POST | `/v1/users/me/open-to-work` | Sí | Marcar como disponible para trabajar |
| DELETE | `/v1/users/me/open-to-work` | Sí | Desmarcar disponibilidad |
| POST | `/v1/users/me/providing-service` | Sí | Marcar como proveedor de servicio |
| DELETE | `/v1/users/me/providing-service` | Sí | Desmarcar proveedor |

---

## Contenido del Usuario

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/users/{username}/posts` | Sí | Posts del usuario |
| GET | `/v1/users/{username}/photos` | Sí | Fotos del usuario |
| GET | `/v1/users/{username}/videos` | Sí | Videos del usuario |
| GET | `/v1/users/{user_id}/common` | Sí | Cosas en común con el usuario |

---

## Configuración

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/users/me/privacy` | Sí | Obtener configuración de privacidad |
| PUT | `/v1/users/me/privacy` | Sí | Actualizar privacidad |
| GET | `/v1/users/me/notification-settings` | Sí | Configuración de notificaciones |
| PUT | `/v1/users/me/notification-settings` | Sí | Actualizar notificaciones |
| GET | `/v1/users/me/invite-code` | Sí | Obtener mi código de invitación |
| GET | `/v1/users/me/fields` | Sí | Valores de campos personalizados |
| PUT | `/v1/users/me/fields` | Sí | Actualizar campos personalizados |
| GET | `/v1/users/{user_id}/fields` | Sí | Campos personalizados de otro usuario |

---

## Direcciones

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/users/me/addresses` | Sí | Listar mis direcciones |
| POST | `/v1/users/me/addresses` | Sí | Crear dirección |
| GET | `/v1/users/me/addresses/{id}` | Sí | Obtener dirección |
| PUT | `/v1/users/me/addresses/{id}` | Sí | Actualizar dirección |
| DELETE | `/v1/users/me/addresses/{id}` | Sí | Eliminar dirección |

---

## Miscelánea

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/users/me/avatar/reset` | Sí | Resetear avatar al predeterminado |
| POST | `/v1/users/me/download-info` | Sí | Exportar datos GDPR |
| PUT | `/v1/users/me/location` | Sí | Actualizar ubicación |
| PUT | `/v1/users/me/lastseen` | Sí | Actualizar última vez visto |
| GET | `/v1/users/me/referrals` | Sí | Mis referidos |
| GET | `/v1/users/me/inviters` | Sí | Mis invitadores |
| POST | `/v1/users/me/onboarding/skip` | Sí | Saltar paso de onboarding |
| POST | `/v1/search/register` | Sí | Registrar búsqueda reciente |
| POST | `/v1/contact` | No | Formulario de contacto |
| POST | `/v1/general` | Sí | Batch fetch de datos (startup móvil) |
| POST | `/v1/reports` | Sí | Crear reporte |
| POST | `/v1/points/admob` | Sí | Registrar puntos AdMob |
| GET | `/v1/activities` | Sí | Listar mis actividades |
