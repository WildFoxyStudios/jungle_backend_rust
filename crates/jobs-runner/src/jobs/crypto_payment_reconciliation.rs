//! Poll pending crypto transactions from Coinbase Commerce / CoinPayments APIs
//! and update their status. Runs every 15 minutes.

use sqlx::PgPool;
use std::time::Duration;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(15 * 60);
    let http = reqwest::Client::new();
    loop {
        if let Err(e) = reconcile_coinbase(&pool, &http).await {
            tracing::error!(error = %e, "crypto_payment_reconciliation (coinbase) failed");
        }
        if let Err(e) = reconcile_coinpayments(&pool, &http).await {
            tracing::error!(error = %e, "crypto_payment_reconciliation (coinpayments) failed");
        }
        tokio::time::sleep(interval).await;
    }
}

async fn reconcile_coinbase(pool: &PgPool, http: &reqwest::Client) -> Result<(), String> {
    let api_key = match std::env::var("COINBASE_COMMERCE_API_KEY").ok() {
        Some(k) if !k.is_empty() => k,
        _ => return Ok(()),
    };

    let pending: Vec<(i64, String)> = sqlx::query_as(
        r#"SELECT id, reference
             FROM transactions
            WHERE provider = 'coinbase'
              AND status = 'pending'
              AND created_at > NOW() - INTERVAL '7 days'
            LIMIT 50"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    for (tx_id, reference) in pending {
        let url = format!("https://api.commerce.coinbase.com/charges/{}", reference);
        let resp = http
            .get(&url)
            .header("X-CC-Api-Key", &api_key)
            .header("X-CC-Version", "2018-03-22")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            continue;
        }

        let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
        let status = body["data"]["timeline"]
            .as_array()
            .and_then(|a| a.last())
            .and_then(|s| s["status"].as_str())
            .unwrap_or("pending")
            .to_lowercase();

        let new_status = match status.as_str() {
            "completed" | "resolved" => Some("completed"),
            "expired" | "canceled" => Some("cancelled"),
            "unresolved" => Some("failed"),
            _ => None,
        };

        if let Some(s) = new_status {
            let _ = sqlx::query(
                "UPDATE transactions SET status = $1, updated_at = NOW() WHERE id = $2",
            )
            .bind(s)
            .bind(tx_id)
            .execute(pool)
            .await;
            tracing::info!(tx_id, status = s, "coinbase tx reconciled");
        }
    }
    Ok(())
}

async fn reconcile_coinpayments(pool: &PgPool, _http: &reqwest::Client) -> Result<(), String> {
    // CoinPayments uses signed POST requests; left as a stub for now.
    // Reconciliation for CoinPayments mainly happens via IPN webhook.
    let key = std::env::var("COINPAYMENTS_PUBLIC_KEY").unwrap_or_default();
    if key.is_empty() {
        return Ok(());
    }

    // Count how many are pending so ops know if they accumulate.
    let pending_count: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM transactions
            WHERE provider = 'coinpayments' AND status = 'pending'
              AND created_at > NOW() - INTERVAL '7 days'"#,
    )
    .fetch_one(pool)
    .await
    .map_err(|e| e.to_string())?;

    if pending_count > 0 {
        tracing::info!(pending_count, "coinpayments: pending txs (awaiting IPN)");
    }
    Ok(())
}
