use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;

use crate::errors::ApiError;

#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn upload(&self, key: &str, data: &[u8], content_type: &str) -> Result<String, ApiError>;
    async fn delete(&self, key: &str) -> Result<(), ApiError>;
    async fn get_url(&self, key: &str) -> String;
}

/// S3-compatible storage (AWS S3, MinIO, Wasabi, DigitalOcean Spaces, Backblaze B2)
pub struct S3Storage {
    client: aws_sdk_s3::Client,
    bucket: String,
    public_url: String,
}

impl S3Storage {
    pub async fn from_env() -> Option<Self> {
        let endpoint = std::env::var("MINIO_ENDPOINT").ok()?;
        let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "wowonder".into());
        let access_key = std::env::var("MINIO_ACCESS_KEY").ok()?;
        let secret_key = std::env::var("MINIO_SECRET_KEY").ok()?;
        let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into());

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
            .region(aws_sdk_s3::config::Region::new(region))
            .endpoint_url(&endpoint)
            .credentials_provider(creds)
            .force_path_style(true)
            .build();

        let client = aws_sdk_s3::Client::from_conf(config);

        Some(Self {
            client,
            public_url: format!("{}/{}", endpoint, bucket),
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
}

/// Create storage provider from env config
pub async fn create_storage() -> Box<dyn StorageProvider> {
    let provider = std::env::var("STORAGE_PROVIDER").unwrap_or_else(|_| "local".into());
    match provider.as_str() {
        "s3" | "minio" | "wasabi" | "spaces" | "backblaze" => {
            if let Some(s3) = S3Storage::from_env().await {
                Box::new(s3)
            } else {
                tracing::warn!("S3 config incomplete, falling back to local storage");
                Box::new(LocalStorage::new("./uploads", "/uploads"))
            }
        }
        _ => Box::new(LocalStorage::new("./uploads", "/uploads")),
    }
}
