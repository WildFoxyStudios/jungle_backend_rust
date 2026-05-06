# Object storage providers (M10-STO-1)

The platform stores user uploads (avatars, post media, files) in
S3-compatible buckets. Multiple providers can be registered through
the admin panel (**Settings → Storage providers**) and the active
ones are tried in `priority` order on every upload.

## Supported providers

| Provider             | `provider_type` | Notes |
|----------------------|-----------------|-------|
| Amazon S3            | `s3`            | Region required, no `endpoint`. |
| Wasabi               | `wasabi`        | `endpoint` = `https://s3.<region>.wasabisys.com`. |
| Backblaze B2 (S3)    | `backblaze`     | `endpoint` = `https://s3.<region>.backblazeb2.com`. |
| DigitalOcean Spaces  | `spaces`        | `endpoint` = `https://<region>.digitaloceanspaces.com`. |
| Cloudflare R2        | `r2`            | `endpoint` = `https://<account>.r2.cloudflarestorage.com`. |
| MinIO (self-hosted)  | `minio`         | Internal `endpoint`, no public URL by default. |
| **Google Cloud Storage** | `s3`        | Use the GCS S3-compatible API: `endpoint = https://storage.googleapis.com`, `region = auto`, HMAC keys from the service account. |

Each entry stores `bucket`, optional `endpoint`, optional `region`,
`access_key`, an encrypted `secret_key`, an optional `public_url` (CDN
domain), and a `priority` (lower = preferred).

## GCS via S3 compatibility

Google Cloud Storage exposes an S3-compatible interoperability layer.
To register a GCS bucket as a storage provider:

1. Open the bucket in the GCP console and enable
   **Interoperability**.
2. Create an HMAC key for the service account that owns the bucket.
3. In the admin panel add a provider with:
   - `provider_type`: `s3`
   - `bucket`: the bucket name
   - `endpoint`: `https://storage.googleapis.com`
   - `region`: `auto`
   - `access_key`: the HMAC access ID
   - `secret_key`: the HMAC secret
   - `public_url` (optional): the public bucket / CDN URL

The `aws-sdk-s3` client used by the test endpoint
(`POST /v1/admin/storage/config/{id}/test`) handles the GCS
endpoint correctly because it issues path-style requests
(`force_path_style: true`).

## Test connection

Every registered provider exposes
`POST /v1/admin/storage/config/{id}/test`. The endpoint performs a
`PutObject` followed by a `DeleteObject` of a temporary `.txt` probe
key and reports success / failure with the underlying error message.

## Encryption at rest

Secret keys are encrypted with the master key derived from
`INTERNAL_SERVICE_KEY` (or `JWT_SECRET` as a fallback) using
`shared::crypto::derive_key` + AEAD. They are decrypted on the
server only when issuing pre-signed URLs or running the test.
