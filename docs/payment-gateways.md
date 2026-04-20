# Payment Gateways

El `payment-service` implementa el trait `PaymentGateway` para 20 proveedores de pago. Todos exponen la misma interfaz.

---

## Trait PaymentGateway

```rust
#[async_trait]
pub trait PaymentGateway: Send + Sync {
    fn provider_name(&self) -> &'static str;
    async fn create_session(&self, params: PaymentParams) -> Result<PaymentSession, PaymentError>;
    async fn verify_payment(&self, reference: &str) -> Result<PaymentStatus, PaymentError>;
    async fn handle_webhook(&self, payload: &[u8], signature: &str) -> Result<WebhookEvent, PaymentError>;
    async fn refund(&self, tx_id: &str, amount: Option<Decimal>) -> Result<RefundResult, PaymentError>;
}
```

---

## Proveedores Implementados

### 1. Stripe
**Variables**: `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`  
**Webhook**: `POST /v1/payments/webhooks/stripe`  
**Características**: Checkout Sessions, webhooks firmados con HMAC-SHA256, reembolsos parciales/totales.

---

### 2. PayPal
**Variables**: `PAYPAL_CLIENT_ID`, `PAYPAL_SECRET`, `PAYPAL_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/paypal`  
**Características**: Orders API v2, sandbox/producción, verificación de webhooks.

---

### 3. Paystack
**Variables**: `PAYSTACK_SECRET_KEY`  
**Webhook**: `POST /v1/payments/webhooks/paystack`  
**Características**: Popular en África (Nigeria, Ghana, Sudáfrica). Verificación HMAC-SHA512.

---

### 4. Flutterwave
**Variables**: `FLUTTERWAVE_SECRET_KEY`  
**Webhook**: `POST /v1/payments/webhooks/flutterwave`  
**Características**: Pagos en África y mercados emergentes. Múltiples métodos de pago locales.

---

### 5. Razorpay
**Variables**: `RAZORPAY_KEY_ID`, `RAZORPAY_KEY_SECRET`  
**Webhook**: `POST /v1/payments/webhooks/razorpay`  
**Características**: Líder en India. Soporta UPI, tarjetas, netbanking, wallets.

---

### 6. Coinbase Commerce
**Variables**: `COINBASE_COMMERCE_API_KEY`  
**Webhook**: `POST /v1/payments/webhooks/coinbase`  
**Características**: Pagos con criptomonedas (BTC, ETH, USDC, etc.). Verificación HMAC-SHA256.

---

### 7. Braintree (PayPal)
**Variables**: `BRAINTREE_MERCHANT_ID`, `BRAINTREE_PUBLIC_KEY`, `BRAINTREE_PRIVATE_KEY`, `BRAINTREE_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/braintree`  
**Características**: Tarjetas de crédito, PayPal, Venmo, Apple Pay, Google Pay.

---

### 8. Bank Transfer (Transferencia Bancaria)
**Variables**: Ninguna (configuración manual)  
**Webhook**: N/A  
**Características**: Genera referencia única `BT-{uuid}`. El admin aprueba manualmente tras verificar el comprobante. Estado inicial siempre `Pending`.

---

### 9. Iyzipay
**Variables**: `IYZIPAY_API_KEY`, `IYZIPAY_SECRET_KEY`, `IYZIPAY_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/iyzipay`  
**Características**: Proveedor turco. Soporta tarjetas locales turcas.

---

### 10. Cashfree
**Variables**: `CASHFREE_APP_ID`, `CASHFREE_SECRET_KEY`, `CASHFREE_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/cashfree`  
**Características**: India. UPI, tarjetas, netbanking, wallets.

---

### 11. YooMoney (YooKassa)
**Variables**: `YOOMONEY_SHOP_ID`, `YOOMONEY_SECRET_KEY`  
**Webhook**: `POST /v1/payments/webhooks/yoomoney`  
**Características**: Rusia. Tarjetas, YooMoney wallet, SBP.

---

### 12. aamarPay
**Variables**: `AAMARPAY_STORE_ID`, `AAMARPAY_SIGNATURE_KEY`, `AAMARPAY_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/aamarpay`  
**Características**: Bangladesh. bKash, Nagad, tarjetas locales.

---

### 13. Fortumo
**Variables**: `FORTUMO_SERVICE_ID`, `FORTUMO_SECRET`  
**Webhook**: `POST /v1/payments/webhooks/fortumo`  
**Características**: Pagos por SMS/carrier billing. Mercados emergentes.

---

### 14. 2Checkout (Verifone)
**Variables**: `TWOCHECKOUT_MERCHANT_CODE`, `TWOCHECKOUT_SECRET_KEY`, `TWOCHECKOUT_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/2checkout`  
**Características**: Global. Suscripciones recurrentes, múltiples monedas.

---

### 15. CoinPayments
**Variables**: `COINPAYMENTS_MERCHANT_ID`, `COINPAYMENTS_PUBLIC_KEY`, `COINPAYMENTS_PRIVATE_KEY`, `COINPAYMENTS_IPN_SECRET`  
**Webhook**: `POST /v1/payments/webhooks/coinpayments`  
**Características**: 100+ criptomonedas. IPN (Instant Payment Notification).

---

### 16. PayFast
**Variables**: `PAYFAST_MERCHANT_ID`, `PAYFAST_MERCHANT_KEY`, `PAYFAST_PASSPHRASE`, `PAYFAST_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/payfast`  
**Características**: Sudáfrica. EFT, tarjetas, Mobicred.

---

### 17. Paysera
**Variables**: `PAYSERA_PROJECT_ID`, `PAYSERA_SIGN_PASSWORD`, `PAYSERA_TEST`  
**Webhook**: `POST /v1/payments/webhooks/paysera`  
**Características**: Europa del Este (Lituania). Transferencias bancarias europeas.

---

### 18. SecurionPay
**Variables**: `SECURIONPAY_SECRET_KEY`  
**Webhook**: `POST /v1/payments/webhooks/securionpay`  
**Características**: Europa. Tarjetas de crédito, 3D Secure.

---

### 19. N-Genius (Network International)
**Variables**: `NGENIUS_API_KEY`, `NGENIUS_OUTLET_REF`, `NGENIUS_SANDBOX`  
**Webhook**: `POST /v1/payments/webhooks/ngenius`  
**Características**: Oriente Medio y África. UAE, Arabia Saudita.

---

### 20. PayPro Bitcoin
**Variables**: `PAYPRO_API_KEY`  
**Webhook**: `POST /v1/payments/webhooks/paypro-bitcoin`  
**Características**: Pagos Bitcoin directos.

---

## Factory de Gateways

```rust
// Crear un gateway por nombre
let gateway = create_gateway("stripe")?;
let session = gateway.create_session(params).await?;
```

El factory `create_gateway(provider: &str)` devuelve `Box<dyn PaymentGateway>` o `PaymentError::UnsupportedProvider`.

---

## Flujo de Pago Típico

```
1. Cliente → POST /v1/payments/create { provider: "stripe", amount: "9.99", ... }
2. payment-service → create_gateway("stripe").create_session(params)
3. Stripe API → devuelve checkout URL
4. payment-service → guarda transacción en DB (estado: pending)
5. Respuesta → { redirect_url: "https://checkout.stripe.com/..." }

6. Usuario completa el pago en Stripe
7. Stripe → POST /v1/payments/webhooks/stripe (webhook)
8. payment-service → verifica firma HMAC
9. payment-service → actualiza transacción en DB (estado: completed)
10. payment-service → publica DomainEvent::PaymentCompleted en NATS
```

---

## Estados de Pago

| Estado | Descripción |
|--------|-------------|
| `Pending` | Pago iniciado, esperando confirmación |
| `Completed` | Pago completado exitosamente |
| `Failed` | Pago fallido |
| `Cancelled` | Pago cancelado por el usuario |
| `Refunded` | Pago reembolsado |
