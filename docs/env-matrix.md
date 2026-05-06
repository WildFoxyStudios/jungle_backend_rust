# Matriz de Entorno (Backend)

Esta matriz resume las variables críticas para operar el backend en `dev`, `staging` y `prod`.

## Variables globales mínimas (producción)

| Variable | Requerida prod | Sensible | Ejemplo |
|---|---|---|---|
| `DATABASE_URL` | sí | sí | `postgresql://...` |
| `REDIS_URL` | sí | sí | `redis://...` |
| `NATS_URL` | sí | no* | `nats://...` |
| `JWT_SECRET` | sí | sí | string aleatorio >= 64 chars |
| `JWT_REFRESH_SECRET` | sí | sí | string aleatorio >= 64 chars |
| `INTERNAL_SERVICE_KEY` | sí | sí | token interno de confianza |
| `ALLOWED_ORIGINS` | sí | no | `https://app.example.com,https://admin.example.com` |
| `FRONTEND_URL` | sí | no | `https://app.example.com` |
| `RUST_LOG` | no | no | `info,sqlx=warn` |

\*Si NATS no es público y está aislado en red privada, puede tratarse como no sensible.

## Persistencia y storage

| Variable | Requerida prod | Sensible | Notas |
|---|---|---|---|
| `STORAGE_PROVIDER` | sí | no | `s3` recomendado en producción |
| `S3_ENDPOINT` | depende | no | requerido para S3-compatible custom |
| `S3_BUCKET` | sí | no | bucket principal |
| `S3_REGION` | sí | no | región cloud |
| `S3_ACCESS_KEY` | sí | sí | credencial storage |
| `S3_SECRET_KEY` | sí | sí | credencial storage |
| `S3_PUBLIC_URL` | recomendado | no | CDN / dominio público assets |

## Email, SMS y push

| Dominio | Variables mínimas |
|---|---|
| SMTP | `SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`, `SMTP_PASSWORD`, `SMTP_FROM_EMAIL` |
| SMS (Twilio) | `SMS_PROVIDER=twilio`, `TWILIO_ACCOUNT_SID`, `TWILIO_AUTH_TOKEN`, `TWILIO_PHONE_FROM` |
| Web Push (VAPID) | `VAPID_PUBLIC_KEY`, `VAPID_PRIVATE_KEY`, `VAPID_SUBJECT` |

## Gateways de pago

No actives todos. Habilita solo los proveedores realmente usados en producción y define sus secretos correspondientes.

Ejemplo mínimo:
- Stripe: `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`
- PayPal: `PAYPAL_CLIENT_ID`, `PAYPAL_SECRET`, `PAYPAL_SANDBOX=false`

## Recomendaciones operativas

1. Mantén secretos fuera de git (Fly secrets / gestor externo).
2. Rota `JWT_SECRET` y usa temporalmente `JWT_SECRET_PREVIOUS`.
3. Versiona cambios de env por entorno en changelog de infraestructura.
4. Nunca uses defaults de localhost en producción.

