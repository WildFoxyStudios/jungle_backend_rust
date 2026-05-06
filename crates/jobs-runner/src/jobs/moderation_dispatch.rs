use sqlx::{PgPool, Row};
use tracing::{error, info, warn};

pub async fn run(pool: PgPool) {
    // Fetch pending items from moderation_queue and classify them via OpenAI.
    let api_key = match std::env::var("OPENAI_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => {
            info!("OPENAI_API_KEY not configured — skipping moderation dispatch");
            return;
        }
    };

    // Get up to 25 pending items
    let items = match sqlx::query(
        "SELECT id, target_type, target_id, content_text, content_image_url
         FROM moderation_queue
         WHERE status = 'pending' AND content_text IS NOT NULL
         ORDER BY created_at
         LIMIT 25"
    )
    .fetch_all(&pool)
    .await
    {
        Ok(rows) => rows,
        Err(e) => {
            error!(error = %e, "Failed to fetch moderation queue");
            return;
        }
    };

    if items.is_empty() {
        return;
    }

    info!(count = items.len(), "Processing moderation queue");

    let client = reqwest::Client::new();

    for item in &items {
        let id: i64 = item.get("id");
        let target_type: String = item.get("target_type");
        let target_id: i64 = item.get("target_id");
        let content_text: String = item.get::<Option<String>, _>("content_text").unwrap_or_default();
        let image_url: Option<String> = item.get("content_image_url");

        if content_text.is_empty() && image_url.is_none() {
            continue;
        }

        // Build the moderation input — use text, or image_url as input
        let input_text = if !content_text.is_empty() {
            content_text.clone()
        } else {
            image_url.clone().unwrap_or_default()
        };

        // Call OpenAI Moderation API
        let body = serde_json::json!({
            "model": "omni-moderation-latest",
            "input": input_text,
        });

        let result = match client
            .post("https://api.openai.com/v1/moderations")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    warn!(queue_id = id, status = %resp.status(), "OpenAI moderation request failed");
                    continue;
                }
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => json,
                    Err(e) => {
                        error!(error = %e, "Failed to parse moderation response");
                        continue;
                    }
                }
            }
            Err(e) => {
                error!(error = %e, "OpenAI moderation request failed");
                continue;
            }
        };

        let mod_result = &result["results"][0];
        let flagged = mod_result["flagged"].as_bool().unwrap_or(false);

        let categories: serde_json::Value = mod_result["categories"].clone();
        let scores: serde_json::Value = mod_result["category_scores"].clone();

        let max_score = scores
            .as_object()
            .map(|obj| {
                obj.values()
                    .filter_map(|v| v.as_f64())
                    .fold(0.0f64, f64::max)
            })
            .unwrap_or(0.0);

        let action = if !flagged {
            "auto_approve"
        } else if max_score > 0.85 {
            "auto_block"
        } else if max_score > 0.5 {
            "human_review"
        } else {
            "auto_approve"
        };

        // Update the queue item with results
        let update_result = sqlx::query(
            "UPDATE moderation_queue
             SET openai_flagged = $1,
                 openai_categories = $2,
                 openai_scores = $3,
                 auto_action = $4,
                 status = CASE WHEN $4 IN ('auto_approve', 'auto_block') THEN $4 ELSE 'human_review' END,
                 resolved_at = CASE WHEN $4 IN ('auto_approve', 'auto_block') THEN NOW() ELSE NULL END
             WHERE id = $5"
        )
        .bind(flagged)
        .bind(&categories)
        .bind(&scores)
        .bind(action)
        .bind(id)
        .execute(&pool)
        .await;

        match update_result {
            Ok(_) => info!(queue_id = id, target_type, target_id, action, "Moderation item processed"),
            Err(e) => error!(error = %e, queue_id = id, "Failed to update moderation item"),
        }
    }

    info!("Moderation dispatch cycle complete");
}
