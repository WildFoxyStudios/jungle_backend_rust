# Plan de Implementación — Flutter Frontend para Jungle

## Análisis del Frontend PHP Existente

El frontend PHP (tema "sunshine") tiene las siguientes secciones principales identificadas:

- **Auth**: welcome, register, login, forgot-password, 2FA, social login
- **Home/Feed**: feed principal, stories, filtros por tipo de post
- **Timeline/Perfil**: cover, avatar, info, posts, seguidores, seguidos
- **Chat/Mensajería**: conversaciones, grupos, llamadas audio/video
- **Notificaciones**: dropdown en header, preferencias
- **Grupos y Páginas**: CRUD, miembros, posts
- **Eventos**: crear, RSVP, asistentes
- **Contenido**: blogs, foros, películas, juegos
- **Comercio**: productos, carrito, pedidos, empleos, crowdfunding
- **Pagos**: wallet, suscripciones Pro, gateways
- **Reels**: feed vertical con swipe
- **Stories**: visualización con temporizador
- **Búsqueda**: global con filtros
- **Configuración**: perfil, privacidad, notificaciones, 2FA, sesiones
- **IA**: generación de posts, blogs, imágenes
- **Admin**: dashboard, moderación, configuración

---

## Stack Flutter Recomendado

```
Flutter 3.x (Dart 3.x)
├── Estado: Riverpod 2.x (AsyncNotifier + StateNotifier)
├── Navegación: go_router 13.x
├── HTTP: dio 5.x + retrofit (code gen)
├── WebSocket: web_socket_channel
├── Cache local: hive + flutter_secure_storage
├── Imágenes: cached_network_image
├── Video: video_player + chewie
├── Push: firebase_messaging (FCM) + flutter_local_notifications
├── Pagos: webview_flutter (redirect gateways)
├── Mapas: flutter_map + geolocator
├── Cámara/Media: image_picker + file_picker
├── Llamadas: agora_rtc_engine
└── Internacionalización: flutter_localizations + intl
```

---

## Estructura del Proyecto Flutter

```
jungle_app/
├── lib/
│   ├── main.dart
│   ├── app.dart                        # MaterialApp + GoRouter setup
│   ├── core/
│   │   ├── api/
│   │   │   ├── api_client.dart         # Dio instance, interceptors
│   │   │   ├── api_constants.dart      # Base URL, endpoints
│   │   │   └── interceptors/
│   │   │       ├── auth_interceptor.dart   # Añade Bearer token
│   │   │       └── retry_interceptor.dart  # Refresh token automático
│   │   ├── auth/
│   │   │   ├── auth_storage.dart       # flutter_secure_storage
│   │   │   └── auth_provider.dart      # Riverpod: token, user actual
│   │   ├── websocket/
│   │   │   └── ws_service.dart         # WebSocket + reconexión
│   │   ├── notifications/
│   │   │   └── push_service.dart       # FCM setup
│   │   ├── theme/
│   │   │   ├── app_theme.dart
│   │   │   └── app_colors.dart
│   │   └── utils/
│   │       ├── date_utils.dart
│   │       ├── media_utils.dart
│   │       └── validators.dart
│   ├── features/
│   │   ├── auth/
│   │   ├── feed/
│   │   ├── profile/
│   │   ├── messaging/
│   │   ├── notifications/
│   │   ├── stories/
│   │   ├── reels/
│   │   ├── search/
│   │   ├── groups/
│   │   ├── pages/
│   │   ├── events/
│   │   ├── blogs/
│   │   ├── forums/
│   │   ├── commerce/
│   │   ├── payments/
│   │   ├── ai/
│   │   └── settings/
│   └── shared/
│       ├── widgets/
│       │   ├── post_card.dart
│       │   ├── user_avatar.dart
│       │   ├── reaction_bar.dart
│       │   └── story_ring.dart
│       └── models/                     # Modelos JSON compartidos
├── test/
├── pubspec.yaml
└── analysis_options.yaml
```

Cada feature sigue la estructura:
```
features/feed/
├── data/
│   ├── feed_repository.dart
│   └── feed_api.dart          # Retrofit interface
├── domain/
│   └── models/
│       ├── post.dart
│       └── feed_response.dart
├── presentation/
│   ├── feed_screen.dart
│   ├── feed_provider.dart     # Riverpod AsyncNotifier
│   └── widgets/
│       └── post_card.dart
└── feed.dart                  # barrel export
```

---

## Fases de Implementación

### FASE 1 — Fundación (Semana 1-2)

**Objetivo**: App funcional con auth, feed básico y navegación.

#### 1.1 Setup del Proyecto
- Crear proyecto Flutter con `flutter create jungle_app`
- Configurar `pubspec.yaml` con todas las dependencias
- Setup de Riverpod, GoRouter, Dio
- Configurar `analysis_options.yaml` (linting estricto)
- Setup de flavors: `dev`, `staging`, `prod`

#### 1.2 Capa de Red (API Client)

```dart
// core/api/api_client.dart
final dioProvider = Provider<Dio>((ref) {
  final dio = Dio(BaseOptions(
    baseUrl: ApiConstants.baseUrl,  // http://localhost:8080
    connectTimeout: const Duration(seconds: 10),
    receiveTimeout: const Duration(seconds: 30),
  ));
  dio.interceptors.addAll([
    AuthInterceptor(ref),
    LogInterceptor(requestBody: true),
  ]);
  return dio;
});
```

**AuthInterceptor** — añade `Authorization: Bearer <token>` y hace refresh automático cuando recibe 401:
```dart
// Si 401 → llama POST /v1/auth/refresh → guarda nuevo token → reintenta
```

#### 1.3 Auth Storage
```dart
// Guarda en flutter_secure_storage:
// - access_token
// - refresh_token
// - user_json (AuthUserResponse serializado)
```

#### 1.4 Pantallas de Auth

| Pantalla | Endpoint | Notas |
|----------|----------|-------|
| Login | `POST /v1/auth/login` | identifier = email/username/phone |
| Registro | `POST /v1/auth/register` | username, email, password, first_name |
| Forgot Password | `POST /v1/auth/forgot-password` | Solo email |
| Reset Password | `POST /v1/auth/reset-password` | token + nueva password |
| Verificar Email | `POST /v1/auth/verify-email` | código de 6 dígitos |
| 2FA Verify | `POST /v1/auth/2fa/verify` | user_id + code |
| Social Login | `POST /v1/auth/social/login` | provider + token |

**Flujo de login con 2FA:**
```
login() → si requires_2fa == true → navegar a TwoFactorScreen
TwoFactorScreen → POST /v1/auth/2fa/verify → guardar tokens → home
```

#### 1.5 Navegación (GoRouter)

```dart
final router = GoRouter(routes: [
  GoRoute(path: '/', redirect: (ctx, state) => authGuard(ctx)),
  GoRoute(path: '/login', builder: (_,__) => LoginScreen()),
  GoRoute(path: '/register', builder: (_,__) => RegisterScreen()),
  ShellRoute(
    builder: (ctx, state, child) => MainShell(child: child),
    routes: [
      GoRoute(path: '/feed', builder: (_,__) => FeedScreen()),
      GoRoute(path: '/reels', builder: (_,__) => ReelsScreen()),
      GoRoute(path: '/messages', builder: (_,__) => MessagesScreen()),
      GoRoute(path: '/notifications', builder: (_,__) => NotificationsScreen()),
      GoRoute(path: '/profile/:username', builder: (_,s) => ProfileScreen(username: s.pathParameters['username']!)),
    ],
  ),
]);
```

#### 1.6 Bottom Navigation Bar

5 tabs principales (igual que el PHP):
1. 🏠 Feed
2. 🎬 Reels
3. ➕ Crear Post (FAB central)
4. 💬 Mensajes
5. 🔔 Notificaciones

Badge en mensajes y notificaciones con conteo de no leídos.

---

### FASE 2 — Feed, Posts y Stories (Semana 3-4)

**Objetivo**: Feed funcional con posts, reacciones, comentarios y stories.

#### 2.1 Feed Screen

```dart
// Cursor-based pagination con Riverpod
class FeedNotifier extends AsyncNotifier<List<Post>> {
  String? _cursor;

  Future<void> loadMore() async {
    final res = await ref.read(feedApiProvider).getFeed(
      cursor: _cursor, limit: 20, filter: 'all',
    );
    _cursor = res.meta.cursor;
    state = AsyncData([...state.value!, ...res.data]);
  }
}
```

Endpoints usados:
- `GET /v1/feed?cursor=&limit=20&filter=all`
- `GET /v1/feed/explore`
- `GET /v1/memories`

**Filtros del feed** (igual que PHP): all, photos, videos, links, polls, live

**Skeleton loading** mientras carga (igual que el PHP tiene `.wo_loading_post`).

#### 2.2 PostCard Widget

Componente central de la app. Incluye:
- Avatar + nombre + tiempo relativo + badge verificado/Pro
- Contenido de texto (con "leer más" si > 3 líneas)
- Media: imagen única, galería (PageView), video con controles
- Colored post (fondo de color con texto)
- Barra de reacciones: like/love/haha/wow/sad/angry
- Contadores: likes, comentarios, compartidos
- Menú de opciones: guardar, ocultar, reportar, editar (si es propio), eliminar
- Indicador de anuncio (`is_ad: true`)

#### 2.3 Crear Post

Pantalla modal con:
- Campo de texto (máx 63,206 chars)
- Selector de privacidad: público, amigos, solo yo
- Adjuntar media: imagen/video desde galería o cámara → `POST /v1/media/upload`
- Feeling/emoción selector
- Ubicación
- Fondo de color (colored post)
- Encuesta (poll) con opciones y duración
- Publicar en grupo/página/evento

Endpoint: `POST /v1/posts`

#### 2.4 Reacciones

```dart
// Long press en botón like → mostrar popup con 6 emojis
// Tap rápido → like/unlike
// POST /v1/posts/{id}/react  { reaction_type: "like" }
// DELETE /v1/posts/{id}/react
```

#### 2.5 Comentarios

- `GET /v1/posts/{id}/comments?cursor=&limit=20` (orden ASC)
- `POST /v1/posts/{id}/comments` { content, media, parent_id }
- Respuestas anidadas un nivel
- Reacciones en comentarios
- Menciones con `@` autocomplete → `GET /v1/mentions?q=`

#### 2.6 Stories

Pantalla de stories al estilo Instagram:
- Barra de progreso por story (5 segundos por defecto)
- Swipe horizontal entre usuarios
- Swipe vertical para cerrar
- Tap izquierdo/derecho para navegar
- Reaccionar con emoji
- Responder (envía DM)
- Ver espectadores (si es propia)

Endpoints:
- `GET /v1/stories` — feed de stories
- `POST /v1/stories/{id}/view`
- `POST /v1/stories/{id}/react`
- `POST /v1/stories/{id}/reply`

Crear story: `POST /v1/stories` con media_id, texto overlay, color de fondo.

#### 2.7 Reels

Feed vertical TikTok-style:
- `PageView` vertical con `PageController`
- Auto-play del video visible, pause al hacer scroll
- Swipe up/down para siguiente/anterior
- Botones laterales: like, comentar, compartir, audio
- Precarga del siguiente reel

Endpoints:
- `GET /v1/reels?cursor=`
- `POST /v1/reels/{id}/view`
- `POST /v1/reels/{id}/react`

---

### FASE 3 — Perfil de Usuario (Semana 5)

**Objetivo**: Timeline completo con cover, avatar, info, posts, seguidores.

#### 3.1 ProfileScreen

Estructura (igual que el PHP `timeline/content.phtml`):
- **Header**: cover photo (editable si es propio) + avatar con ring de story
- **Info**: nombre, @username, badge verificado, badge Pro, bio, ubicación, web
- **Botones de acción**:
  - Si es propio: Editar perfil, Actividades
  - Si es otro: Seguir/Dejar de seguir, Mensaje, Notificar, Más (bloquear, reportar)
- **Tabs**: Posts, Fotos, Videos, Amigos, Grupos, Páginas, Álbumes
- **Contadores**: posts, seguidores, seguidos

Endpoints:
- `GET /v1/users/{username}` — perfil completo
- `GET /v1/users/{username}/popover` — hover card ligero
- `GET /v1/users/{username}/followers`
- `GET /v1/users/{username}/following`
- `GET /v1/users/{username}/posts`
- `GET /v1/users/{username}/photos`
- `GET /v1/users/{username}/videos`

#### 3.2 Editar Perfil

Formulario con:
- Nombre, apellido, username, bio
- Género, cumpleaños, ciudad, web, escuela, trabajo
- Idioma preferido
- Links sociales (Facebook, Twitter, Instagram, etc.)

Endpoints:
- `PUT /v1/users/me`
- `PUT /v1/users/me/social-links`
- `PUT /v1/users/me/avatar` (tras subir con `POST /v1/media/upload/avatar`)
- `PUT /v1/users/me/cover`

#### 3.3 Grafo Social

```dart
// Botón de seguir con estados:
// - "Seguir" → POST /v1/social/follow/{user_id}
// - "Pendiente" (perfil privado) → estado pending
// - "Siguiendo" → DELETE /v1/social/follow/{user_id}
```

Pantallas adicionales:
- Lista de seguidores/seguidos con paginación
- Solicitudes de seguimiento pendientes
- Usuarios bloqueados
- Usuarios silenciados

#### 3.4 Perfil Profesional (modo LinkedIn)

Si el backend tiene `website_mode = linkedin`:
- Sección de experiencia laboral
- Certificaciones
- Proyectos
- Habilidades con autocomplete
- Badge "Open to Work"
- Badge "Providing Service"

---

### FASE 4 — Mensajería en Tiempo Real (Semana 6-7)

**Objetivo**: Chat completo con WebSocket, grupos, llamadas.

#### 4.1 Lista de Conversaciones

- `GET /v1/conversations` — lista con último mensaje
- Conversaciones fijadas arriba
- Badge de no leídos por conversación
- Swipe para archivar/fijar/eliminar
- Búsqueda de conversaciones

#### 4.2 Chat Screen

```dart
// Mensajes en orden DESC (más recientes abajo)
// Cursor pagination hacia arriba (cargar más antiguos)
// GET /v1/conversations/{id}/messages?cursor=&limit=20
```

Tipos de mensaje soportados:
- Texto con menciones y links
- Imagen (tap para ver en fullscreen)
- Video con reproductor inline
- Audio con waveform y botón play/pause
- Sticker
- GIF
- Ubicación (mapa estático)
- Archivo adjunto

Funcionalidades:
- Responder a mensaje (reply preview)
- Reenviar mensaje
- Reaccionar con emoji (toggle)
- Fijar mensaje
- Marcar como favorito
- Eliminar mensaje (solo propio)
- Indicador de escritura (typing dots)
- Marcar como leído al entrar

#### 4.3 WebSocket Integration

```dart
// core/websocket/ws_service.dart
class WsService {
  late WebSocketChannel _channel;

  void connect(String token) {
    _channel = WebSocketChannel.connect(
      Uri.parse('ws://localhost:8080/ws?token=$token'),
    );
    _channel.stream.listen(_onMessage, onDone: _reconnect);
  }

  void _onMessage(dynamic data) {
    final msg = jsonDecode(data);
    switch (msg['event']) {
      case 'new_message':    // → actualizar conversación
      case 'typing_start':   // → mostrar "escribiendo..."
      case 'typing_stop':    // → ocultar typing
      case 'notification':   // → badge + local notification
      case 'presence_online': // → punto verde en avatar
      case 'incoming_call':  // → mostrar pantalla de llamada entrante
    }
  }
}
```

Reconexión automática con backoff exponencial (1s, 2s, 4s, 8s...).

#### 4.4 Typing Indicator

```dart
// Al escribir → POST /v1/conversations/{id}/typing (debounced 2s)
// Al recibir typing_start → mostrar "Juan está escribiendo..."
// Auto-ocultar tras 3s (TTL del Redis en backend)
```

#### 4.5 Grupos de Chat

- Crear grupo: nombre, imagen, participantes
- `POST /v1/conversations/group`
- Gestionar miembros: añadir/eliminar
- Cambiar nombre e imagen del grupo
- Salir del grupo

#### 4.6 Llamadas Audio/Video (Agora)

```dart
// Flujo de llamada saliente:
// 1. POST /v1/calls { callee_id, call_type: "video" }
// 2. POST /v1/calls/agora-token → obtener token Agora
// 3. Inicializar agora_rtc_engine con el token
// 4. Mostrar CallingScreen

// Flujo de llamada entrante (WebSocket):
// 1. Recibir evento incoming_call
// 2. Mostrar IncomingCallScreen (igual que modals/calling.phtml)
// 3. Aceptar → PUT /v1/calls/{id}/status { status: "accepted" }
// 4. Rechazar → PUT /v1/calls/{id}/status { status: "rejected" }
```

Pantallas:
- `CallingScreen` — marcando, esperando respuesta
- `InCallScreen` — en llamada con controles (mute, cámara, altavoz, colgar)
- `IncomingCallScreen` — aceptar/rechazar con animación de ring

---

### FASE 5 — Notificaciones (Semana 7)

**Objetivo**: Notificaciones in-app y push.

#### 5.1 Notificaciones In-App

- `GET /v1/notifications?cursor=&limit=20`
- `GET /v1/notifications/unread-count` — badge en tab
- `POST /v1/notifications/read-all`
- `POST /v1/notifications/{id}/read`

Tipos de notificación con iconos distintos:
- 👍 post_like — "Juan reaccionó a tu post"
- 💬 post_comment — "María comentó en tu post"
- 👤 follow — "Pedro te siguió"
- 🎂 birthday — "Hoy es el cumpleaños de Ana"
- 📅 event_reminder — "Tu evento empieza en 1 hora"
- 💳 payment — "Pago completado"
- 📢 mention — "Te mencionaron en un post"

#### 5.2 Push Notifications (FCM)

```dart
// push_service.dart
Future<void> setupPush() async {
  final token = await FirebaseMessaging.instance.getToken();
  // Registrar token en backend:
  await api.registerPushToken({
    'token': token,
    'platform': Platform.isAndroid ? 'android' : 'ios',
    'device_id': await getDeviceId(),
  });
  // POST /v1/notifications/push-tokens
}
```

Manejar notificaciones en foreground con `flutter_local_notifications`.
Al tocar la notificación → navegar a la pantalla correcta según el tipo.

#### 5.3 Preferencias de Notificaciones

- `GET /v1/notifications/preferences`
- `PUT /v1/notifications/preferences`
- Toggle por tipo: likes, comentarios, follows, mensajes, cumpleaños, etc.

---

### FASE 6 — Búsqueda y Descubrimiento (Semana 8)

#### 6.1 Búsqueda Global

```
GET /v1/search?q=rust&type=user&cursor=&limit=20
```

Tabs de resultados: Usuarios, Posts, Páginas, Grupos, Hashtags, Blogs, Productos

Búsquedas recientes:
- `GET /v1/search/recent`
- `DELETE /v1/search/recent` — limpiar

#### 6.2 Hashtags

- `GET /v1/hashtags/trending` — lista de trending
- `GET /v1/hashtags/{tag}/posts` — posts por hashtag
- `GET /v1/hashtags/search?q=` — autocomplete al escribir #

#### 6.3 Usuarios Sugeridos

- `GET /v1/users/suggestions` — sugerencias de seguimiento
- `GET /v1/users/nearby` — usuarios cercanos (requiere permiso de ubicación)
- `GET /v1/users/birthdays` — amigos con cumpleaños hoy

#### 6.4 Directorio

Pantalla de exploración con tabs:
- Usuarios, Páginas, Grupos, Eventos, Blogs, Productos, Empleos, Películas, Juegos

---

### FASE 7 — Grupos, Páginas y Eventos (Semana 9)

#### 7.1 Grupos

Pantallas:
- Lista de grupos (mis grupos, sugeridos, unidos)
- Detalle del grupo: cover, avatar, descripción, posts, miembros
- Crear grupo: nombre, slug, categoría, privacidad, descripción
- Configuración del grupo: general, privacidad, miembros, solicitudes, analytics

Endpoints clave:
- `POST /v1/groups/{id}/join` / `DELETE /v1/groups/{id}/join`
- `GET /v1/groups/{id}/members`
- `GET /v1/groups/{id}/join-requests` + accept/reject

#### 7.2 Páginas

Similar a grupos pero con:
- Sistema de likes en lugar de join
- Calificación (1-5 estrellas)
- Admins de página
- Boost de página (Pro)
- Solicitud de verificación

#### 7.3 Eventos

- Lista: próximos, mis eventos, asistiendo
- Detalle: cover, fecha, ubicación, descripción, asistentes
- RSVP: going / interested / not_going
- Crear evento con fecha, hora, ubicación, privacidad
- Invitar usuarios

---

### FASE 8 — Contenido (Semana 10)

#### 8.1 Blogs

- Lista con categorías y búsqueda
- Lector de blog con HTML renderizado (`flutter_widget_from_html`)
- Crear/editar blog con editor rich text (`flutter_quill`)
- Subir imagen para el editor: `POST /v1/blogs/upload-image`
- Comentarios y reacciones en blogs
- Generación con IA: `POST /v1/ai/generate-blog`

#### 8.2 Foros

- Secciones → Foros → Hilos → Respuestas
- Crear hilo con título y contenido
- Votar en hilos
- Compartir hilo

#### 8.3 Películas

- Lista con filtros por género y país
- Reproductor de video con `video_player`
- Comentarios y reacciones

#### 8.4 Juegos

- Lista de juegos disponibles
- Abrir juego en `WebView` (los juegos son HTML5)
- Registrar partida: `POST /v1/games/{id}/play`

---

### FASE 9 — Comercio y Pagos (Semana 11-12)

#### 9.1 Marketplace de Productos

- Lista con búsqueda, categorías, productos cercanos
- Detalle del producto: imágenes (galería), descripción, precio, reseñas
- Crear/editar producto con múltiples imágenes
- Carrito: añadir, actualizar cantidad, vaciar
- Checkout → crear pedido → pago

Endpoints:
- `GET /v1/products?cursor=&limit=20`
- `POST /v1/cart` / `GET /v1/cart`
- `POST /v1/orders`

#### 9.2 Empleos

- Lista con búsqueda y categorías
- Detalle del empleo: empresa, descripción, salario, tipo
- Aplicar con CV/mensaje: `POST /v1/jobs/{id}/apply`
- Mis aplicaciones: `GET /v1/jobs/applied`

#### 9.3 Crowdfunding

- Lista de campañas activas
- Detalle: progreso, donaciones, descripción
- Donar: `POST /v1/fundings/{id}/donate` → flujo de pago

#### 9.4 Sistema de Pagos

**Flujo general:**
```
1. Usuario elige acción de pago (Pro, donación, producto)
2. Seleccionar gateway (Stripe, PayPal, etc.)
3. POST /v1/payments/create → obtener redirect_url
4. Abrir WebView con redirect_url
5. Gateway redirige a return_url o cancel_url
6. App detecta la URL → POST /v1/payments/verify
7. Mostrar resultado
```

```dart
// payments/payment_webview.dart
class PaymentWebView extends StatelessWidget {
  final String redirectUrl;
  final String returnUrl;

  @override
  Widget build(BuildContext context) {
    return WebViewWidget(
      controller: WebViewController()
        ..setNavigationDelegate(NavigationDelegate(
          onNavigationRequest: (req) {
            if (req.url.startsWith(returnUrl)) {
              // Pago completado
              context.pop(PaymentResult.success);
              return NavigationDecision.prevent;
            }
            return NavigationDecision.navigate;
          },
        ))
        ..loadRequest(Uri.parse(redirectUrl)),
    );
  }
}
```

**Wallet:**
- `GET /v1/payments/wallet/balance`
- `POST /v1/payments/wallet/add` → flujo de pago
- `POST /v1/payments/wallet/transfer` → transferir a usuario

**Suscripción Pro:**
- `GET /v1/payments/pro/plans` — mostrar planes
- `POST /v1/payments/pro/subscribe` → flujo de pago
- Pantalla "Go Pro" con beneficios y planes

**Creator (Patreon):**
- Ver tiers de un creator
- Suscribirse a un creator
- Gestionar mis tiers (si soy creator)

#### 9.5 Regalos y Stickers

- `GET /v1/gifts` — catálogo de regalos
- `POST /v1/gifts/send/{recipient_id}` — enviar regalo
- `GET /v1/stickers/packs` — packs disponibles
- `POST /v1/stickers/packs/{id}/purchase` — comprar pack

---

### FASE 10 — IA, Configuración y Ajustes (Semana 13)

#### 10.1 Funciones de IA

Integradas en el flujo natural de la app:

- **Crear Post**: botón ✨ → `POST /v1/ai/generate-post` → rellena el campo de texto
- **Crear Blog**: botón ✨ → `POST /v1/ai/generate-blog` → rellena título y contenido
- **Generar Imagen**: `POST /v1/ai/generate-images` → seleccionar para adjuntar al post
- **Describir Imagen**: al subir imagen → `POST /v1/ai/describe-image` → alt text automático
- **Balance de créditos**: `GET /v1/ai/balance/words` y `/images`

Pantalla de créditos IA (igual que `modals/ai_credits.phtml`):
- Créditos de texto restantes / límite
- Créditos de imágenes restantes / límite
- Fecha de reset

#### 10.2 Configuración del Usuario

Pantalla de settings con secciones (igual que `setting/content.phtml`):

| Sección | Endpoints |
|---------|-----------|
| Perfil general | `PUT /v1/users/me` |
| Cambiar contraseña | `PUT /v1/auth/password` |
| Privacidad | `GET/PUT /v1/users/me/privacy` |
| Notificaciones | `GET/PUT /v1/users/me/notification-settings` |
| Sesiones activas | `GET /v1/auth/sessions` + DELETE |
| 2FA | Setup, enable, disable, backup codes |
| Direcciones | CRUD `/v1/users/me/addresses` |
| Campos personalizados | `GET/PUT /v1/users/me/fields` |
| Historial de pagos | `GET /v1/payments/history` |
| Wallet | Balance + transacciones |
| Monetización | Tiers de creator |
| Mis puntos | Balance de puntos |
| Código de invitación | `GET /v1/users/me/invite-code` |
| Exportar datos (GDPR) | `POST /v1/users/me/download-info` |
| Eliminar cuenta | `DELETE /v1/users/me` |
| Usuarios bloqueados | `GET /v1/social/blocked` |
| Links sociales | `PUT /v1/users/me/social-links` |
| Experiencia/Certificaciones | CRUD profesional |

#### 10.3 Onboarding

Para nuevos usuarios (igual que `start_up/`):
1. Completar info básica (nombre, género, cumpleaños)
2. Subir avatar
3. Seguir usuarios sugeridos

`POST /v1/users/me/onboarding/skip` para saltar pasos.

---

### FASE 11 — Live Streaming (Semana 14)

#### 11.1 Iniciar Live

```dart
// 1. POST /v1/live/start → obtener stream_id y token Agora
// 2. Inicializar agora_rtc_engine como broadcaster
// 3. Mostrar LiveBroadcastScreen con:
//    - Preview de cámara
//    - Contador de espectadores
//    - Comentarios en tiempo real (WebSocket)
//    - Botón de reacciones flotantes
// 4. POST /v1/live/stop al terminar
```

#### 11.2 Ver Live

```dart
// GET /v1/live/active — lista de lives activos
// GET /v1/live/friends — amigos en live
// Al entrar: inicializar Agora como audience
// Comentar: POST /v1/live/{id}/comment
// Reaccionar: POST /v1/live/{id}/react
```

Pantalla de live viewer:
- Video a pantalla completa
- Comentarios superpuestos (scroll automático)
- Reacciones flotantes animadas
- Contador de espectadores
- Botón de regalo virtual

---

### FASE 12 — Pulido y Producción (Semana 15-16)

#### 12.1 Temas y Personalización

```dart
// Soporte dark/light mode
// Color primario configurable desde /v1/config/public
// Fuentes y tamaños accesibles
```

#### 12.2 Internacionalización

- `GET /v1/translations/{lang}` — cargar traducciones del backend
- Guardar idioma preferido localmente
- Soporte RTL (árabe, hebreo) con `Directionality` widget

#### 12.3 Offline Support

- Cache de feed con Hive (últimos 50 posts)
- Cache de conversaciones recientes
- Indicador de "sin conexión" con banner
- Cola de acciones pendientes (posts, reacciones) para sincronizar al reconectar

#### 12.4 Performance

- Lazy loading de imágenes con `cached_network_image`
- Precarga de siguiente página del feed
- `RepaintBoundary` en PostCard para evitar repaints innecesarios
- `const` constructors donde sea posible
- Dispose correcto de VideoPlayerController y WebSocketChannel

#### 12.5 Seguridad

- Tokens en `flutter_secure_storage` (no SharedPreferences)
- Certificate pinning con Dio
- Ofuscación de código en release: `flutter build apk --obfuscate`
- No loguear tokens ni datos sensibles en producción

---

## pubspec.yaml Completo

```yaml
name: jungle_app
description: Jungle Social Network
version: 1.0.0+1

environment:
  sdk: '>=3.0.0 <4.0.0'
  flutter: '>=3.10.0'

dependencies:
  flutter:
    sdk: flutter

  # Estado
  flutter_riverpod: ^2.5.1
  riverpod_annotation: ^2.3.5

  # Navegación
  go_router: ^13.2.0

  # Red
  dio: ^5.4.3
  retrofit: ^4.1.0

  # WebSocket
  web_socket_channel: ^2.4.0

  # Storage
  flutter_secure_storage: ^9.0.0
  hive_flutter: ^1.1.0

  # Imágenes
  cached_network_image: ^3.3.1
  image_picker: ^1.1.2
  image_cropper: ^5.0.1

  # Video
  video_player: ^2.8.6
  chewie: ^1.8.1

  # Push
  firebase_messaging: ^14.9.4
  flutter_local_notifications: ^17.2.2

  # Pagos
  webview_flutter: ^4.7.0

  # Mapas
  flutter_map: ^6.1.0
  geolocator: ^11.0.0
  latlong2: ^0.9.0

  # Llamadas
  agora_rtc_engine: ^6.3.2

  # UI
  shimmer: ^3.0.0
  flutter_staggered_grid_view: ^0.7.0
  photo_view: ^0.14.0
  emoji_picker_flutter: ^2.2.0
  flutter_widget_from_html: ^0.15.1
  flutter_quill: ^9.4.4
  timeago: ^3.6.1
  lottie: ^3.1.2

  # Utilidades
  freezed_annotation: ^2.4.1
  json_annotation: ^4.9.0
  intl: ^0.19.0
  url_launcher: ^6.3.0
  share_plus: ^9.0.0
  path_provider: ^2.1.3
  connectivity_plus: ^6.0.3
  device_info_plus: ^10.1.0
  package_info_plus: ^8.0.0
  file_picker: ^8.0.3
  open_file: ^3.3.2
  permission_handler: ^11.3.1

dev_dependencies:
  flutter_test:
    sdk: flutter
  build_runner: ^2.4.9
  riverpod_generator: ^2.4.0
  retrofit_generator: ^8.1.0
  freezed: ^2.5.2
  json_serializable: ^6.8.0
  hive_generator: ^2.0.1
  flutter_lints: ^4.0.0
  mocktail: ^1.0.3
```

---

## Modelos de Datos Principales (Dart)

```dart
// shared/models/user.dart
@freezed
class User with _$User {
  const factory User({
    required int id,
    required String uuid,
    required String username,
    required String email,
    required String firstName,
    required String lastName,
    required String name,
    required String avatar,
    required String cover,
    required String about,
    required bool isVerified,
    required int isPro,
    required bool isAdmin,
    required bool twoFactorEnabled,
    required bool emailVerified,
    String? location,
    String? website,
    String? birthday,
  }) = _User;

  factory User.fromJson(Map<String, dynamic> json) => _$UserFromJson(json);
}

// shared/models/post.dart
@freezed
class Post with _$Post {
  const factory Post({
    required int id,
    required String uuid,
    required int userId,
    required String content,
    required String postType,
    required dynamic media,
    required String privacy,
    required String feeling,
    required String location,
    required bool isPinned,
    required bool isBoosted,
    required bool isReel,
    required int likeCount,
    required int commentCount,
    required int shareCount,
    required int viewCount,
    required DateTime createdAt,
    User? publisher,
    bool? isAd,
    int? adId,
  }) = _Post;

  factory Post.fromJson(Map<String, dynamic> json) => _$PostFromJson(json);
}

// shared/models/message.dart
@freezed
class Message with _$Message {
  const factory Message({
    required int id,
    required int conversationId,
    required int senderId,
    required String senderUsername,
    required String senderFirstName,
    required String senderLastName,
    required String senderAvatar,
    required String content,
    required String messageType,
    required dynamic media,
    int? replyToId,
    int? forwardedFrom,
    required bool isPinned,
    required bool isFavorited,
    required DateTime createdAt,
  }) = _Message;

  factory Message.fromJson(Map<String, dynamic> json) => _$MessageFromJson(json);
}
```

---

## Resumen de Pantallas por Fase

| Fase | Pantallas | Semanas |
|------|-----------|---------|
| 1 — Fundación | Login, Register, ForgotPassword, ResetPassword, VerifyEmail, 2FA, SocialLogin, MainShell | 1-2 |
| 2 — Feed | FeedScreen, ReelsScreen, CreatePost, PostDetail, CommentsSheet, StoriesViewer, CreateStory | 3-4 |
| 3 — Perfil | ProfileScreen, EditProfile, FollowersList, FollowingList, BlockedUsers | 5 |
| 4 — Mensajería | ConversationsList, ChatScreen, CreateGroup, IncomingCall, InCallScreen | 6-7 |
| 5 — Notificaciones | NotificationsScreen, NotificationPreferences | 7 |
| 6 — Búsqueda | SearchScreen, HashtagPosts, UserSuggestions, NearbyUsers | 8 |
| 7 — Grupos/Páginas/Eventos | GroupDetail, CreateGroup, PageDetail, EventDetail, CreateEvent | 9 |
| 8 — Contenido | BlogList, BlogReader, CreateBlog, ForumList, ThreadDetail, MovieList, GamesList | 10 |
| 9 — Comercio/Pagos | ProductList, ProductDetail, Cart, Checkout, JobsList, FundingList, PaymentWebView, WalletScreen, GoProScreen | 11-12 |
| 10 — IA/Settings | AiPostGenerator, AiCredits, SettingsScreen (todas las subsecciones) | 13 |
| 11 — Live | LiveBroadcast, LiveViewer, ActiveLives | 14 |
| 12 — Pulido | Temas, i18n, offline, performance, seguridad | 15-16 |

**Total estimado: 16 semanas (4 meses) para un equipo de 2-3 desarrolladores Flutter.**

---

## Conexión con el Backend

### URL Base
```dart
// core/api/api_constants.dart
class ApiConstants {
  static const String baseUrl = 'http://localhost:8080';
  static const String wsUrl = 'ws://localhost:8080/ws';

  // En producción:
  // static const String baseUrl = 'https://api.tudominio.com';
  // static const String wsUrl = 'wss://api.tudominio.com/ws';
}
```

### Headers Requeridos
```
Authorization: Bearer <access_token>   // en todas las rutas protegidas
Content-Type: application/json
```

### Paginación (cursor-based)
```dart
// Todas las listas usan el mismo patrón:
// GET /v1/feed?cursor=122&limit=20
// Response: { data: [...], meta: { cursor: "101", has_more: true } }

class PaginatedResponse<T> {
  final List<T> data;
  final PaginationMeta meta;
}

class PaginationMeta {
  final String? cursor;
  final bool hasMore;
  final int? total;
}
```

### Manejo de Errores
```dart
// Todos los errores tienen el formato:
// { error: { code: "NOT_FOUND", message: "..." } }
// { error: { code: "VALIDATION_ERROR", message: "...", details: [...] } }

class ApiException implements Exception {
  final String code;
  final String message;
  final List<FieldError>? details;
}
```

### Refresh Token Automático
```dart
// auth_interceptor.dart
@override
Future onError(DioException err, ErrorInterceptorHandler handler) async {
  if (err.response?.statusCode == 401) {
    try {
      final newToken = await _refreshToken();
      // Reintentar request original con nuevo token
      err.requestOptions.headers['Authorization'] = 'Bearer $newToken';
      final response = await dio.fetch(err.requestOptions);
      return handler.resolve(response);
    } catch (_) {
      // Refresh falló → logout
      ref.read(authProvider.notifier).logout();
    }
  }
  return handler.next(err);
}
```

---

## Archivos de Configuración Iniciales

### pubspec.yaml (sección flutter)
```yaml
flutter:
  uses-material-design: true
  assets:
    - assets/images/
    - assets/animations/    # Lottie JSON
    - assets/fonts/
  fonts:
    - family: Inter
      fonts:
        - asset: assets/fonts/Inter-Regular.ttf
        - asset: assets/fonts/Inter-Medium.ttf
          weight: 500
        - asset: assets/fonts/Inter-SemiBold.ttf
          weight: 600
        - asset: assets/fonts/Inter-Bold.ttf
          weight: 700
```

### android/app/build.gradle
```groovy
android {
    defaultConfig {
        minSdkVersion 21      // Para Agora y WebView
        targetSdkVersion 34
    }
}
```

### ios/Runner/Info.plist (permisos)
```xml
<key>NSCameraUsageDescription</key>
<string>Para subir fotos y hacer videollamadas</string>
<key>NSMicrophoneUsageDescription</key>
<string>Para llamadas de audio y video</string>
<key>NSPhotoLibraryUsageDescription</key>
<string>Para subir fotos y videos</string>
<key>NSLocationWhenInUseUsageDescription</key>
<string>Para mostrar usuarios y productos cercanos</string>
```

---

## Comandos para Empezar

```bash
# 1. Crear el proyecto
flutter create jungle_app --org com.jungle --platforms android,ios

# 2. Instalar dependencias
cd jungle_app
flutter pub get

# 3. Generar código (freezed, json_serializable, retrofit, riverpod)
dart run build_runner build --delete-conflicting-outputs

# 4. Arrancar el backend (en otra terminal)
cd ../backend
docker compose up -d

# 5. Correr la app
flutter run --flavor dev

# 6. Verificar que el backend responde
curl http://localhost:8080/health
```

---

## Notas Importantes

1. **El backend ya está completo** — 519 endpoints listos para consumir. No hay que esperar nada del lado del servidor.

2. **WebSocket** — conectar al arrancar la app si el usuario está autenticado. Reconectar automáticamente si se pierde la conexión.

3. **Tokens JWT** — el access token dura 15 minutos. El interceptor de Dio debe hacer refresh automático antes de que expire (o cuando recibe 401).

4. **Imágenes** — siempre subir primero con `POST /v1/media/upload` y usar el `id` devuelto en los posts/stories/etc.

5. **Paginación** — nunca usar offset. Siempre cursor-based. El cursor es el ID del último elemento.

6. **Rate limiting** — el backend limita `/v1/auth/*` a 10 req/15min. Mostrar mensaje amigable si se recibe 429.

7. **Modo Instagram** — el backend soporta `website_mode = instagram`. En ese modo, el feed es solo de seguidos y el perfil es más visual.

8. **Swagger UI** — disponible en `http://localhost:8080/swagger-ui` para explorar todos los endpoints durante el desarrollo.
