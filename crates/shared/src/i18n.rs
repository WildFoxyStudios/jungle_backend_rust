use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type LangMap = HashMap<String, String>;

#[derive(Clone)]
pub struct I18nService {
    cache: Arc<RwLock<HashMap<String, LangMap>>>,
    db: PgPool,
}

impl I18nService {
    pub async fn new(db: PgPool) -> Self {
        let svc = Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            db,
        };
        svc.load_all().await.ok();
        svc
    }

    async fn load_all(&self) -> Result<(), sqlx::Error> {
        let rows = sqlx::query_as::<_, (String, String, String)>(
            "SELECT lang, key, value FROM translations",
        )
        .fetch_all(&self.db)
        .await?;

        let mut map: HashMap<String, LangMap> = HashMap::new();
        for (lang, key, value) in rows {
            map.entry(lang).or_default().insert(key, value);
        }
        *self.cache.write().await = map;
        Ok(())
    }

    /// Translate a key for the given language, falling back to English
    pub async fn t(&self, lang: &str, key: &str) -> String {
        let cache = self.cache.read().await;
        cache
            .get(lang)
            .and_then(|m| m.get(key))
            .or_else(|| cache.get("english").and_then(|m| m.get(key)))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// Get all translations for a language
    pub async fn get_all_for_lang(&self, lang: &str) -> LangMap {
        self.cache.read().await.get(lang).cloned().unwrap_or_default()
    }

    /// List all available languages
    pub async fn available_languages(&self) -> Vec<String> {
        self.cache.read().await.keys().cloned().collect()
    }

    /// Force reload from database
    pub async fn reload(&self) {
        self.load_all().await.ok();
    }
}

/// Extract language from Accept-Language header or user preference
pub fn extract_language(headers: &axum::http::HeaderMap, user_lang: Option<&str>) -> String {
    user_lang
        .map(String::from)
        .or_else(|| {
            headers
                .get("accept-language")
                .and_then(|v| v.to_str().ok())
                .map(|s| {
                    s.split(',')
                        .next()
                        .unwrap_or("english")
                        .split('-')
                        .next()
                        .unwrap_or("english")
                        .to_string()
                })
        })
        .unwrap_or_else(|| "english".to_string())
}
