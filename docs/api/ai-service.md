# API — AI Service (Puerto 3013)

El AI Service integra múltiples proveedores con **fallback automático** y un sistema de **créditos por usuario**.

## Sistema de Proveedores

Los proveedores se configuran dinámicamente en la tabla `ai_provider_config` (no solo por variables de entorno). El sistema selecciona el proveedor con mayor prioridad disponible para cada capacidad.

**Capacidades**: `text`, `image`, `both`

**Proveedores soportados**: `openai`, `anthropic`, `gemini`

---

## Endpoints de Usuario

### POST /v1/ai/generate-post

Genera contenido para un post de redes sociales.

```json
// Request
{
  "prompt": "Escribe sobre el lanzamiento de Rust 2.0",
  "topic": "tecnología",
  "tone": "casual",
  "max_tokens": 300
}

// Response
{
  "data": {
    "content": "¡Rust 2.0 ya está aquí! 🦀 El lenguaje más amado...",
    "provider": "openai",
    "model": "gpt-4o-mini",
    "tokens_used": 87
  }
}
```

Tonos disponibles: `casual`, `professional`, `funny`, `inspirational`, `informative`

---

### POST /v1/ai/generate-blog

Genera un artículo de blog completo en Markdown.

```json
// Request
{
  "topic": "Introducción a Rust para desarrolladores Python",
  "keywords": ["ownership", "borrow checker", "performance"],
  "tone": "informative",
  "length": "medium"
}

// Response
{
  "data": {
    "title": "Introducción a Rust para desarrolladores Python",
    "content": "# Introducción a Rust...\n\n## ¿Por qué Rust?\n...",
    "provider": "openai",
    "model": "gpt-4o-mini",
    "tokens_used": 1250
  }
}
```

Longitudes: `short` (~600 palabras), `medium` (~1200 palabras), `long` (~2000 palabras)

---

### POST /v1/ai/generate-images

Genera imágenes a partir de un prompt.

```json
// Request
{
  "prompt": "Un robot programando en Rust, estilo cyberpunk, colores neón",
  "n": 1,
  "size": "1024x1024",
  "quality": "standard",
  "style": "vivid"
}

// Response
{
  "data": {
    "urls": ["https://cdn.example.com/ai/generated_abc123.png"],
    "provider": "openai",
    "model": "dall-e-3"
  }
}
```

- `n`: 1-4 imágenes
- `size`: `256x256`, `512x512`, `1024x1024`, `1024x1792`, `1792x1024`
- `quality`: `standard`, `hd`
- `style`: `vivid`, `natural`

---

### POST /v1/ai/chat (legacy)

Chat conversacional genérico.

```json
// Request
{
  "prompt": "¿Qué es el borrow checker de Rust?",
  "system_prompt": "Eres un experto en Rust. Responde en español.",
  "max_tokens": 500,
  "temperature": 0.7
}

// Response
{
  "data": {
    "reply": "El borrow checker es un componente del compilador...",
    "provider": "openai"
  }
}
```

---

### POST /v1/ai/suggest-post (legacy)

Alias de `generate-post` para compatibilidad con el cliente PHP.

```json
// Request
{
  "context": "tecnología",
  "content_type": "casual"
}
```

---

### POST /v1/ai/describe-image

Genera descripción de accesibilidad para una imagen (alt-text). Usa GPT-4o-mini Vision. Máximo 200 caracteres.

```json
// Request
{ "image_url": "https://cdn.example.com/photos/sunset.jpg" }

// Response
{
  "data": {
    "description": "Atardecer sobre el mar con tonos naranjas y rojos"
  }
}
```

---

### GET /v1/ai/balance/words

Créditos de texto disponibles.

```json
{
  "data": {
    "remaining": 4500,
    "limit": 5000,
    "plan": "free",
    "reset_at": "2026-05-01T00:00:00Z"
  }
}
```

---

### GET /v1/ai/balance/images

Créditos de imágenes disponibles.

```json
{
  "data": {
    "remaining": 8,
    "limit": 10,
    "plan": "free",
    "reset_at": "2026-05-01T00:00:00Z"
  }
}
```

---

## Endpoints de Admin

Todos requieren `is_admin: true`.

### GET /v1/admin/ai/providers

Lista los proveedores configurados. Las API keys se muestran enmascaradas.

```json
{
  "data": [
    {
      "id": 1,
      "name": "OpenAI Primary",
      "provider_type": "openai",
      "capability": "both",
      "api_key_masked": "sk-...abc123",
      "model_text": "gpt-4o-mini",
      "model_image": "dall-e-3",
      "enabled": true,
      "priority": 1
    }
  ]
}
```

### POST /v1/admin/ai/providers

Crear proveedor. La API key se cifra con AES-GCM antes de almacenarse.

```json
{
  "name": "Anthropic Fallback",
  "provider_type": "anthropic",
  "capability": "text",
  "api_key": "sk-ant-...",
  "model_text": "claude-3-5-sonnet-20241022",
  "enabled": true,
  "priority": 2
}
```

### PUT /v1/admin/ai/providers/{id}

Actualizar proveedor (campos opcionales).

### DELETE /v1/admin/ai/providers/{id}

Eliminar proveedor.

### POST /v1/admin/ai/providers/{id}/test

Prueba el proveedor enviando un prompt mínimo ("Reply with the single word: OK").

```json
// Response éxito
{ "data": { "ok": true, "reply": "OK", "provider": "openai" } }

// Response fallo
{ "data": { "ok": false, "error": "API key invalid" } }
```

### GET /v1/ai/admin/providers/health

Snapshot del estado de los proveedores configurados.

```json
{
  "data": {
    "providers": [
      { "provider_type": "openai", "capability": "both", "priority": 1 },
      { "provider_type": "anthropic", "capability": "text", "priority": 2 }
    ],
    "coverage": {
      "text": true,
      "image": true
    }
  }
}
```

---

## Sistema de Créditos

Los créditos se gestionan en la tabla `ai_credits`:

| Tipo | Descripción |
|------|-------------|
| `Words(n)` | Deduce `n` palabras estimadas del saldo |
| `Images(n)` | Deduce `n` imágenes del saldo |

Si el usuario no tiene créditos suficientes, el endpoint retorna `402 Payment Required`.

Los créditos se resetean mensualmente. Los planes Pro tienen límites más altos.

Cada uso se registra en `ai_usage_log` con: proveedor, tipo, tokens usados, éxito/fallo.

---

## Seguridad de API Keys

Las API keys de los proveedores se almacenan **cifradas con AES-GCM** en la columna `api_key_encrypted`. La clave de cifrado se configura via variable de entorno `AI_ENCRYPTION_KEY`. Nunca se exponen en texto plano en las respuestas de la API.
