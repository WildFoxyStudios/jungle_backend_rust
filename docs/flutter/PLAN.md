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
