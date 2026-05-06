# SMS providers (M10-SMS-1)

The platform supports three SMS gateways for OTP / 2FA / phone verification.
The active provider is selected from the admin panel under
**Settings → SMS** and persisted in `site_config` under the `sms` category.

## Supported providers

| Provider | `sms.provider` | Required keys |
|----------|----------------|---------------|
| Disabled | `disabled`     | — (no SMS sent; phone signups fall back to email)
| Twilio   | `twilio`       | `twilio_account_sid`, `twilio_auth_token`, `twilio_from_number`
| Infobip  | `infobip`      | `infobip_api_key`, `infobip_base_url`
| MSG91    | `msg91`        | `msg91_auth_key`, `msg91_sender_id`

All keys live in the `sms` category of `site_config` and are exposed
through the catalog form
(`backend/crates/admin-service/src/handlers/config_catalog.rs`).

## Wire format

Outgoing SMS messages are short transactional bodies (OTPs, security
alerts). The platform never sends marketing SMS, so the provider only
needs `outgoing_text_send` permissions on its API key.

## Adding a new provider

1. Add the field definitions to the `sms` category of
   `config_catalog.rs` (`provider` select option + secret fields).
2. Implement the gateway under
   `backend/crates/auth-service/src/sms/<provider>.rs` and dispatch
   from a `Provider` enum loaded at runtime from `site_config`.
3. Document the keys in this file.

## Regional notes

- **Twilio** — required `from` number must be E.164 and a verified
  long-code / shortcode in the destination country.
- **Infobip** — `infobip_base_url` is region-scoped, e.g.
  `https://api.infobip.com` for global vs. a regional pod URL.
- **MSG91** — best for India; `msg91_sender_id` is the DLT-registered
  6-character sender ID.

## Failure handling

When the provider returns a non-2xx response the auth handler logs the
provider error and falls back to a generic `sms.send_failed` problem
detail. Failed sends are not retried — the user sees a single error
and can ask for a fresh code.
