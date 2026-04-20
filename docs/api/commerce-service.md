# API — Commerce Service (Puerto 3009)

---

## Productos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/products` | Sí | Listar productos |
| POST | `/v1/products` | Sí | Crear producto |
| GET | `/v1/products/search` | Sí | Buscar productos |
| GET | `/v1/products/my` | Sí | Mis productos |
| GET | `/v1/products/categories` | No | Listar categorías |
| GET | `/v1/products/{id}` | Sí | Obtener producto |
| PUT | `/v1/products/{id}` | Sí | Actualizar producto |
| DELETE | `/v1/products/{id}` | Sí | Eliminar producto |
| GET | `/v1/products/{id}/reviews` | Sí | Reseñas del producto |
| POST | `/v1/products/{id}/reviews` | Sí | Agregar reseña |
| POST | `/v1/products/nearby` | Sí | Productos cercanos |

### POST /v1/products

```json
{
  "name": "Camiseta Rust",
  "description": "Camiseta de alta calidad",
  "price": "29.99",
  "currency": "USD",
  "category_id": 4,
  "media_ids": [10, 11],
  "stock": 100,
  "condition": "new",
  "shipping": true
}
```

Condición: `new`, `used`, `refurbished`

---

## Carrito

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/cart` | Sí | Obtener carrito |
| POST | `/v1/cart` | Sí | Agregar item al carrito |
| DELETE | `/v1/cart` | Sí | Vaciar carrito |
| PUT | `/v1/cart/{id}` | Sí | Actualizar cantidad |
| DELETE | `/v1/cart/{id}` | Sí | Eliminar item del carrito |

---

## Pedidos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/orders` | Sí | Crear pedido |
| GET | `/v1/orders/my` | Sí | Mis pedidos (como comprador) |
| GET | `/v1/orders/sales` | Sí | Mis ventas (como vendedor) |
| GET | `/v1/orders/{id}` | Sí | Obtener pedido |
| PUT | `/v1/orders/{id}/status` | Sí | Actualizar estado del pedido |
| GET | `/v1/orders/{id}/tracking` | Sí | Seguimiento del pedido |
| POST | `/v1/orders/{id}/refund` | Sí | Solicitar reembolso |

### PUT /v1/orders/{id}/status

```json
{ "status": "shipped", "tracking_number": "ES123456789" }
```

Estados: `pending`, `confirmed`, `processing`, `shipped`, `delivered`, `cancelled`, `refunded`

---

## Empleos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/jobs` | Sí | Listar empleos |
| POST | `/v1/jobs` | Sí | Crear oferta de empleo |
| GET | `/v1/jobs/my` | Sí | Mis ofertas de empleo |
| GET | `/v1/jobs/applied` | Sí | Empleos a los que apliqué |
| GET | `/v1/jobs/search` | Sí | Buscar empleos |
| GET | `/v1/jobs/categories` | No | Categorías de empleos |
| GET | `/v1/jobs/{id}` | Sí | Obtener empleo |
| PUT | `/v1/jobs/{id}` | Sí | Actualizar empleo |
| DELETE | `/v1/jobs/{id}` | Sí | Eliminar empleo |
| POST | `/v1/jobs/{id}/apply` | Sí | Aplicar a empleo |
| GET | `/v1/jobs/{id}/applications` | Sí | Listar aplicaciones |
| PUT | `/v1/jobs/applications/{id}/status` | Sí | Actualizar estado de aplicación |

### POST /v1/jobs

```json
{
  "title": "Desarrollador Rust Senior",
  "description": "Buscamos...",
  "category_id": 2,
  "location": "Madrid (Remoto)",
  "salary_min": 50000,
  "salary_max": 80000,
  "currency": "EUR",
  "type": "full_time",
  "experience": "senior"
}
```

Tipos: `full_time`, `part_time`, `freelance`, `internship`

---

## Crowdfunding

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/fundings` | Sí | Listar campañas |
| POST | `/v1/fundings` | Sí | Crear campaña |
| GET | `/v1/fundings/my` | Sí | Mis campañas |
| GET | `/v1/fundings/{id}` | Sí | Obtener campaña |
| PUT | `/v1/fundings/{id}` | Sí | Actualizar campaña |
| DELETE | `/v1/fundings/{id}` | Sí | Eliminar campaña |
| POST | `/v1/fundings/{id}/donate` | Sí | Donar a campaña |
| GET | `/v1/fundings/{id}/donations` | Sí | Listar donaciones |

---

## Ofertas

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/offers` | Sí | Listar ofertas |
| POST | `/v1/offers` | Sí | Crear oferta |
| GET | `/v1/offers/my` | Sí | Mis ofertas |
| GET | `/v1/offers/nearby` | Sí | Ofertas cercanas |
| GET | `/v1/offers/{id}` | Sí | Obtener oferta |
| PUT | `/v1/offers/{id}` | Sí | Actualizar oferta |
| DELETE | `/v1/offers/{id}` | Sí | Eliminar oferta |

---

## Regalos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/gifts` | Sí | Listar regalos disponibles |
| GET | `/v1/gifts/categories` | No | Categorías de regalos |
| POST | `/v1/gifts/send/{recipient_id}` | Sí | Enviar regalo a usuario |
| GET | `/v1/gifts/received` | Sí | Mis regalos recibidos |

---

## Stickers

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/stickers/packs` | Sí | Listar packs de stickers |
| GET | `/v1/stickers/packs/{id}` | Sí | Obtener pack |
| POST | `/v1/stickers/packs/{id}/purchase` | Sí | Comprar pack |
| GET | `/v1/stickers/my` | Sí | Mis packs comprados |
