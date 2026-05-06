//! Poll pending crypto transactions from Coinbase Commerce / CoinPayments and
//! advance their status. Runs every 15 minutes.
//!
//! Each provider is reconciled independently — failure of one does not block
//! the other. The final outcome (best of both) is what gets reported to
//! `cronjob_runs` so the admin UI surfaces health correctly.

use hmac::{Hmac, Mac};
use sha2::Sha512;
use sqlx::PgPool;
use std::collections::HashMap;
use std::time::Duration;

use crate::cron;

type HmacSha512 = Hmac<Sha512>;

pub async fn run(pool: PgPool) {
    let interval = Duration::from_secs(15 * 60);
    let http = reqwest::Client::new();
    loop {
        cron::tracked(&pool, "crypto_payment_reconciliation", || async {
            let coinbase = reconcile_coinbase(&pool, &http).await;
            let coinpayments = reconcile_coinpayments(&pool, &http).await;

            // Surface both numbers; if either errored, fail the run so admins notice.
            match (coinbase, coinpayments) {
                (Ok(cb), Ok(cp)) => Ok(format!("coinbase {}, coinpayments {}", cb, cp)),
                (Err(e), Ok(cp)) => Err(format!("coinbase: {} (coinpayments {})", e, cp)),
                (Ok(cb), Err(e)) => Err(format!("coinpayments: {} (coinbase {})", e, cb)),
                (Err(e1), Err(e2)) => Err(format!("coinbase: {}; coinpayments: {}", e1, e2)),
            }
        })
        .await;

        tokio::time::sleep(interval).await;
    }
}

// ── Coinbase Commerce ──────────────────────────────────────────────

async fn reconcile_coinbase(pool: &PgPool, http: &reqwest::Client) -> Result<u32, String> {
    let api_key = match std::env::var("COINBASE_COMMERCE_API_KEY").ok() {
        Some(k) if !k.is_empty() => k,
        _ => return Ok(0),
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

    let mut updated = 0u32;
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

        if let Some(s) = new_status
            && sqlx::query("UPDATE transactions SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(s)
                .bind(tx_id)
                .execute(pool)
                .await
                .is_ok()
        {
            updated += 1;
            tracing::info!(tx_id, status = s, "coinbase tx reconciled");
        }
    }
    Ok(updated)
}

// ── CoinPayments ───────────────────────────────────────────────────

async fn reconcile_coinpayments(pool: &PgPool, http: &reqwest::Client) -> Result<u32, String> {
    let public_key = std::env::var("COINPAYMENTS_PUBLIC_KEY").unwrap_or_default();
    let private_key = std::env::var("COINPAYMENTS_PRIVATE_KEY").unwrap_or_default();
    if public_key.is_empty() || private_key.is_empty() {
        return Ok(0);
    }

    let pending: Vec<(i64, String)> = sqlx::query_as(
        r#"SELECT id, reference
             FROM transactions
            WHERE provider = 'coinpayments'
              AND status = 'pending'
              AND created_at > NOW() - INTERVAL '7 days'
            LIMIT 50"#,
    )
    .fetch_all(pool)
    .await
    .map_err(|e| e.to_string())?;

    let mut updated = 0u32;
    for (tx_id, txn_id) in pending {
        let info = match cp_get_tx_info(http, &public_key, &private_key, &txn_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(tx_id, error = %e, "coinpayments get_tx_info failed");
                continue;
            }
        };

        // CoinPayments status codes: <0 failed/cancelled, 0..99 pending,
        // >=100 completed. See https://www.coinpayments.net/apidoc-get-tx-info
        let status_code = info["status"].as_i64().unwrap_or(-1);
        let new_status = if status_code >= 100 {
            Some("completed")
        } else if status_code < 0 {
            Some("failed")
        } else {
            None
        };

        if let Some(s) = new_status
            && sqlx::query("UPDATE transactions SET status = $1, updated_at = NOW() WHERE id = $2")
                .bind(s)
                .bind(tx_id)
                .execute(pool)
                .await
                .is_ok()
        {
            updated += 1;
            tracing::info!(tx_id, status = s, "coinpayments tx reconciled");
        }
    }

    Ok(updated)
}

async fn cp_get_tx_info(
    http: &reqwest::Client,
    public_key: &str,
    private_key: &str,
    txn_id: &str,
) -> Result<serde_json::Value, String> {
    let mut form: HashMap<String, String> = HashMap::new();
    form.insert("version".into(), "1".into());
    form.insert("key".into(), public_key.to_string());
    form.insert("cmd".into(), "get_tx_info".into());
    form.insert("format".into(), "json".into());
    form.insert("txid".into(), txn_id.to_string());

    let body = serde_urlencoded::to_string(&form).map_err(|e| e.to_string())?;

    let mut mac = HmacSha512::new_from_slice(private_key.as_bytes())
        .map_err(|_| "invalid HMAC key".to_string())?;
    mac.update(body.as_bytes());
    let hmac_sig = hex::encode(mac.finalize().into_bytes());

    let resp = http
        .post("https://www.coinpayments.net/api.php")
        .header("HMAC", hmac_sig)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    if json["error"].as_str() != Some("ok") {
        return Err(json["error"]
            .as_str()
            .unwrap_or("CoinPayments error")
            .to_string());
    }
    Ok(json["result"].clone())
}
