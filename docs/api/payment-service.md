# API — Payment Service (Puerto 3011)

Ver también: [Payment Gateways](../payment-gateways.md) para la documentación detallada de cada proveedor.

---

## Pagos

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/payments/create` | Sí | Crear sesión de pago |
| POST | `/v1/payments/verify` | Sí | Verificar pago |
| GET | `/v1/payments/history` | Sí | Historial de pagos |
| POST | `/v1/payments/refund` | Sí | Solicitar reembolso |

### POST /v1/payments/create

```json
{
  "provider": "stripe",
  "amount": "29.99",
  "currency": "USD",
  "payment_type": "pro_subscription",
  "return_url": "https://app.example.com/payment/success",
  "cancel_url": "https://app.example.com/payment/cancel"
}
```

```json
// Response 201
{
  "data": {
    "session_id": "cs_test_...",
    "redirect_url": "https://checkout.stripe.com/...",
    "provider": "stripe"
  }
}
```

### POST /v1/payments/verify

```json
{
  "provider": "stripe",
  "reference": "cs_test_..."
}
```

---

## Wallet

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/payments/wallet/balance` | Sí | Obtener saldo del wallet |
| POST | `/v1/payments/wallet/add` | Sí | Agregar fondos al wallet |
| POST | `/v1/payments/wallet/transfer` | Sí | Transferir a otro usuario |

### POST /v1/payments/wallet/transfer

```json
{
  "recipient_id": 42,
  "amount": "10.00",
  "currency": "USD",
  "note": "Gracias!"
}
```

---

## Retiros

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/payments/withdraw` | Sí | Solicitar retiro |
| GET | `/v1/payments/withdrawals` | Sí | Listar mis retiros |
| PUT | `/v1/payments/withdrawals/{id}/status` | Sí | Actualizar estado del retiro |

---

## Suscripciones Pro

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| GET | `/v1/payments/pro/plans` | No | Listar planes Pro |
| POST | `/v1/payments/pro/subscribe` | Sí | Suscribirse a plan Pro |
| POST | `/v1/payments/pro/cancel` | Sí | Cancelar suscripción Pro |
| POST | `/v1/payments/pro/refund-request` | Sí | Solicitar reembolso Pro |
| POST | `/v1/payments/bank-receipt` | Sí | Subir comprobante de transferencia bancaria |

---

## Suscripciones Creator (Modo Patreon)

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/payments/creator/tiers` | Sí | Crear tier de creator |
| PUT | `/v1/payments/creator/tiers/{id}` | Sí | Actualizar tier |
| DELETE | `/v1/payments/creator/tiers/{id}` | Sí | Eliminar tier |
| GET | `/v1/payments/creator/{user_id}/tiers` | Sí | Listar tiers del creator |
| POST | `/v1/payments/creator/subscribe/{user_id}` | Sí | Suscribirse a creator |
| DELETE | `/v1/payments/creator/subscribe/{user_id}` | Sí | Cancelar suscripción |
| GET | `/v1/payments/creator/subscribers` | Sí | Mis suscriptores |
| GET | `/v1/payments/creator/subscriptions` | Sí | Mis suscripciones activas |

---

## Webhooks

| Método | Ruta | Auth | Descripción |
|--------|------|------|-------------|
| POST | `/v1/payments/webhooks/{provider}` | No | Webhook de pago |

Proveedores soportados en webhooks:
`stripe`, `paypal`, `paystack`, `coinbase`, `flutterwave`, `razorpay`, `cashfree`, `iyzipay`, `yoomoney`, `aamarpay`, `fortumo`, `coinpayments`, `2checkout`, `braintree`, `payfast`, `paysera`, `securionpay`, `ngenius`, `paypro-bitcoin`

Cada proveedor verifica la firma del webhook antes de procesar el evento.
