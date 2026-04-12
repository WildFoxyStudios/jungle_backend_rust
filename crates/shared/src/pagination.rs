use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

impl PaginationParams {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(20).clamp(1, 100)
    }

    pub fn cursor_id(&self) -> Option<i64> {
        self.cursor.as_ref().and_then(|c| c.parse::<i64>().ok())
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    pub cursor: Option<String>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(mut items: Vec<T>, limit: i64, total: Option<i64>) -> Self {
        let has_more = items.len() as i64 > limit;
        if has_more {
            items.pop();
        }
        PaginatedResponse {
            data: items,
            meta: PaginationMeta {
                cursor: None,
                has_more,
                total,
            },
        }
    }

    pub fn with_cursor(mut self, cursor: Option<String>) -> Self {
        self.meta.cursor = cursor;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_defaults() {
        let params = PaginationParams {
            cursor: None,
            limit: None,
        };
        assert_eq!(params.limit(), 20);
        assert_eq!(params.cursor_id(), None);
    }

    #[test]
    fn test_pagination_params_clamp() {
        let params = PaginationParams {
            cursor: None,
            limit: Some(500),
        };
        assert_eq!(params.limit(), 100);

        let params = PaginationParams {
            cursor: None,
            limit: Some(0),
        };
        assert_eq!(params.limit(), 1);
    }

    #[test]
    fn test_pagination_params_cursor() {
        let params = PaginationParams {
            cursor: Some("42".into()),
            limit: None,
        };
        assert_eq!(params.cursor_id(), Some(42));

        let params = PaginationParams {
            cursor: Some("invalid".into()),
            limit: None,
        };
        assert_eq!(params.cursor_id(), None);
    }

    #[test]
    fn test_paginated_response_has_more() {
        let items: Vec<i32> = (0..=20).collect();
        let resp = PaginatedResponse::new(items, 20, Some(50));
        assert_eq!(resp.data.len(), 20);
        assert!(resp.meta.has_more);
        assert_eq!(resp.meta.total, Some(50));
    }

    #[test]
    fn test_paginated_response_no_more() {
        let items: Vec<i32> = (0..10).collect();
        let resp = PaginatedResponse::new(items, 20, None);
        assert_eq!(resp.data.len(), 10);
        assert!(!resp.meta.has_more);
    }
}
