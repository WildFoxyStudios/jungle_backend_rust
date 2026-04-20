use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;

use crate::errors::ApiError;

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, ApiError>;
    async fn delete(&self, key: &str) -> Result<(), ApiError>;
    async fn get_url(&self, key: &str) -> String;

    /// Download a previously uploaded object. The default implementation
    /// performs an HTTP GET against the public URL which works for any
    /// provider serving public objects (R2/S3/MinIO with public bucket policies
    /// or local fs via the dev server). Implementations SHOULD override this
    /// with a native SDK call when possible for better performance and to
    /// support private objects.
    async fn download(&self, key: &str) -> Result<Vec<u8>, ApiError> {
        let url = self.get_url(key).await;
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| ApiError::Internal(format!("download {key}: {e}")))?;
        if !resp.status().is_success() {
            return Err(ApiError::NotFound(format!(
                "download {key}: HTTP {}",
                resp.status()
            )));
        }
        Ok(resp
            .bytes()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .to_vec())
    }
}

/// S3-compatible storage (AWS S3, MinIO, Wasabi, DigitalOcean Spaces, Backblaze B2, Cloudflare R2)
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    bucket: String,
    public_url: String,
}

impl S3Storage {
    pub async fn from_env() -> Option<Self> {
        let endpoint = std::env::var("S3_ENDPOINT")
            .or_else(|_| std::env::var("MINIO_ENDPOINT"))
            .ok()?;
        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "jungle".into());
        let access_key = std::env::var("S3_ACCESS_KEY")
            .or_else(|_| std::env::var("MINIO_ACCESS_KEY"))
            .ok()?;
        let secret_key = std::env::var("S3_SECRET_KEY")
            .or_else(|_| std::env::var("MINIO_SECRET_KEY"))
            .ok()?;
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "auto".into());

        if access_key.is_empty() || secret_key.is_empty() {
            return None;
        }

        let creds = aws_sdk_s3::config::Credentials::new(
            &access_key,
            &secret_key,
            None,
            None,
            "env",
        );

        let config = aws_sdk_s3::Config::builder()
            .behavior_version(aws_sdk_s3::config::BehaviorVersion::latest())
            .region(aws_sdk_s3::config::Region::new(region))
            .endpoint_url(&endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();

        let client = aws_sdk_s3::Client::from_conf(config);

        // S3_PUBLIC_URL allows a separate public-facing URL (e.g. R2 custom domain, CloudFront, r2.dev)
        let public_url = std::env::var("S3_PUBLIC_URL")
            .unwrap_or_else(|_| format!("{}/{}", endpoint, bucket));

        Some(Self {
            client,
            public_url,
            bucket,
        })
    }
}

#[async_trait]
impl StorageProvider for S3Storage {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, ApiError> {
        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(ByteStream::from(data.to_vec()))
            .content_type(content_type)
            .cache_control("public, max-age=31536000, immutable")
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("S3 upload failed: {}", e)))?;

        Ok(format!("{}/{}", self.public_url, key))
    }

    async fn delete(&self, key: &str) -> Result<(), ApiError> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("S3 delete failed: {}", e)))?;

        Ok(())
    }

    async fn get_url(&self, key: &str) -> String {
        format!("{}/{}", self.public_url, key)
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, ApiError> {
        let output = self
            .client
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ApiError::Internal(format!("S3 download failed: {}", e)))?;
        let data = output
            .body
            .collect()
            .await
            .map_err(|e| ApiError::Internal(format!("S3 body collect failed: {}", e)))?;
        Ok(data.into_bytes().to_vec())
    }
}

/// Local filesystem storage (development)
pub struct LocalStorage {
    pub base_path: String,
    pub base_url: String,
}

impl LocalStorage {
    pub fn new(base_path: &str, base_url: &str) -> Self {
        Self {
            base_path: base_path.to_string(),
            base_url: base_url.to_string(),
        }
    }
}

#[async_trait]
impl StorageProvider for LocalStorage {
    async fn upload(&self, key: &str, data: &[u8], _content_type: &str) -> Result<String, ApiError> {
        let path = std::path::Path::new(&self.base_path).join(key);
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;
        }
        tokio::fs::write(&path, data)
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

        Ok(format!("{}/{}", self.base_url, key))
    }

    async fn delete(&self, key: &str) -> Result<(), ApiError> {
        let path = std::path::Path::new(&self.base_path).join(key);
        tokio::fs::remove_file(path).await.ok();
        Ok(())
    }

    async fn get_url(&self, key: &str) -> String {
        format!("{}/{}", self.base_url, key)
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, ApiError> {
        let path = std::path::Path::new(&self.base_path).join(key);
        tokio::fs::read(&path)
            .await
            .map_err(|e| ApiError::NotFound(format!("download {key}: {e}")))
    }
}

/// Create storage provider from env config
pub async fn create_storage() -> Box<dyn StorageProvider> {
    let provider = std::env::var("STORAGE_PROVIDER").unwrap_or_else(|_| "r2".into());
    match provider.as_str() {
        "s3" | "minio" | "wasabi" | "spaces" | "backblaze" | "r2" | "cloudflare" => {
            if let Some(s3) = S3Storage::from_env().await {
                tracing::info!(provider = %provider, "Using S3-compatible storage");
                Box::new(s3) as Box<dyn StorageProvider>
            } else {
                tracing::warn!(provider = %provider, "S3 config incomplete, falling back to local storage");
                Box::new(LocalStorage::new("./uploads", "/uploads")) as Box<dyn StorageProvider>
            }
        }
        "local" => {
            tracing::info!("Using local filesystem storage");
            Box::new(LocalStorage::new("./uploads", "/uploads")) as Box<dyn StorageProvider>
        }
        _ => {
            tracing::warn!(provider = %provider, "Unknown storage provider, falling back to local");
            Box::new(LocalStorage::new("./uploads", "/uploads")) as Box<dyn StorageProvider>
        }
    }
}
