use axum::{Json, extract::State};
use serde::Deserialize;
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};

#[derive(Debug, Deserialize)]
pub struct SubscribeRequest {
    pub plan_type: i16,
    pub period: String,
}

#[derive(Debug, Deserialize)]
pub struct RefundRequest {
    pub pro_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BankReceiptRequest {
    pub receipt_file: String,
    pub price: rust_decimal::Decimal,
    pub description: Option<String>,
    pub mode: Option<String>,
}

/// Map a plan name ("star"/"hot"/"ultima"/"vip") to its numeric tier (1-4)
/// used by `subscribe` and the client. Unknown names return 0.
fn plan_type_to_tier(plan_type: &str) -> i16 {
    match plan_type.to_ascii_lowercase().as_str() {
        "star" => 1,
        "hot" => 2,
        "ultima" => 3,
        "vip" => 4,
        _ => 0,
    }
}

fn tier_to_plan_type(tier: i16) -> Option<&'static str> {
    match tier {
        1 => Some("star"),
        2 => Some("hot"),
        3 => Some("ultima"),
        4 => Some("vip"),
        _ => None,
    }
}

/// GET /v1/payments/pro/plans — return all active pro plans grouped by
/// tier (Star/Hot/Ultima/VIP) with monthly + yearly prices pulled from the
/// `pro_plans` table (managed via admin panel). Falls back to sensible
/// defaults if no rows exist yet (fresh install / migration in progress).
pub async fn list_plans(State(state): State<AppState>) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query_as::<_, (String, String, rust_decimal::Decimal, i32, bool)>(
        "SELECT plan_type, title, price, period_days, is_active \
           FROM pro_plans \
          WHERE is_active = TRUE \
          ORDER BY price ASC",
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    use std::collections::BTreeMap;
    #[derive(Default)]
    struct Tier {
        name: String,
        monthly: Option<rust_decimal::Decimal>,
        yearly: Option<rust_decimal::Decimal>,
    }
    let mut tiers: BTreeMap<i16, Tier> = BTreeMap::new();

    for (plan_type, title, price, period_days, _active) in rows {
        let tier = plan_type_to_tier(&plan_type);
        if tier == 0 {
            continue;
        }
        let entry = tiers.entry(tier).or_default();
        if entry.name.is_empty() {
            entry.name = title;
        }
        // 30-day window → monthly; 365-day window → yearly.
        // Other values are ignored for the discover endpoint but still
        // billable via `subscribe`.
        if (1..=45).contains(&period_days) {
            entry.monthly = Some(price);
        } else if (300..=400).contains(&period_days) {
            entry.yearly = Some(price);
        }
    }

    // Fallback defaults (only used when admin hasn't configured rows yet).
    let defaults: &[(i16, &str, f64, f64)] = &[
        (1, "Star", 4.99, 49.99),
        (2, "Hot", 9.99, 99.99),
        (3, "Ultima", 14.99, 149.99),
        (4, "VIP", 29.99, 299.99),
    ];

    let data: Vec<Value> = defaults
        .iter()
        .map(|(tier, default_name, default_monthly, default_yearly)| {
            let configured = tiers.get(tier);
            let name = configured
                .and_then(|t| (!t.name.is_empty()).then(|| t.name.clone()))
                .unwrap_or_else(|| (*default_name).to_string());
            let monthly_price = configured
                .and_then(|t| t.monthly)
                .map(|d| d.to_string())
                .unwrap_or_else(|| default_monthly.to_string());
            let yearly_price = configured
                .and_then(|t| t.yearly)
                .map(|d| d.to_string())
                .unwrap_or_else(|| default_yearly.to_string());
            json!({
                "type": tier,
                "name": name,
                "monthly_price": monthly_price,
                "yearly_price": yearly_price,
            })
        })
        .collect();

    Ok(Json(json!({ "data": data })))
}

pub async fn subscribe(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<SubscribeRequest>,
) -> Result<Json<Value>, ApiError> {
    if !(1..=4).contains(&req.plan_type) {
        return Err(ApiError::BadRequest("Invalid plan type (1-4)".into()));
    }

    // Current pricing matrix only supports monthly/yearly periods.
    let valid_periods = ["monthly", "yearly"];
    if !valid_periods.contains(&req.period.as_str()) {
        return Err(ApiError::BadRequest(format!(
            "Invalid period. Use: {}",
            valid_periods.join(", ")
        )));
    }

    let duration_days: i64 = match req.period.as_str() {
        "monthly" => 30,
        "yearly" => 365,
        _ => {
            return Err(ApiError::BadRequest(
                "Unsupported subscription period".into(),
            ));
        }
    };

    let plan_type_name = tier_to_plan_type(req.plan_type).ok_or_else(|| {
        ApiError::BadRequest("Unsupported plan/period combination".into())
    })?;

    // Preferred path: read price from the admin-managed `pro_plans` table.
    // Match the closest row with `plan_type = name` and period_days in the
    // expected window so admins can tweak 30/31/365/366 without breaking.
    let (window_min, window_max) = match req.period.as_str() {
        "monthly" => (1, 45),
        "yearly" => (300, 400),
        _ => unreachable!(),
    };

    let db_price: Option<rust_decimal::Decimal> = sqlx::query_scalar(
        "SELECT price FROM pro_plans \
          WHERE plan_type = $1 \
            AND is_active = TRUE \
            AND period_days BETWEEN $2 AND $3 \
          ORDER BY ABS(period_days - $4) ASC \
          LIMIT 1",
    )
    .bind(plan_type_name)
    .bind(window_min)
    .bind(window_max)
    .bind(duration_days as i32)
    .fetch_optional(&state.db)
    .await?;

    // Fallback prices used only if the admin has not configured plans yet.
    // Kept intentionally explicit so behaviour stays deterministic on a
    // fresh install.
    let fallback_price = match (req.plan_type, req.period.as_str()) {
        (1, "monthly") => rust_decimal::Decimal::new(499, 2),
        (1, "yearly") => rust_decimal::Decimal::new(4999, 2),
        (2, "monthly") => rust_decimal::Decimal::new(999, 2),
        (2, "yearly") => rust_decimal::Decimal::new(9999, 2),
        (3, "monthly") => rust_decimal::Decimal::new(1499, 2),
        (3, "yearly") => rust_decimal::Decimal::new(14999, 2),
        (4, "monthly") => rust_decimal::Decimal::new(2999, 2),
        (4, "yearly") => rust_decimal::Decimal::new(29999, 2),
        _ => {
            return Err(ApiError::BadRequest(
                "Unsupported plan/period combination".into(),
            ));
        }
    };

    let price = db_price.unwrap_or(fallback_price);

    let mut tx = state.db.begin().await?;

    // Check balance
    let balance = sqlx::query_scalar::<_, rust_decimal::Decimal>(
        "SELECT COALESCE(balance, 0) FROM users WHERE id = $1 FOR UPDATE",
    )
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;

    if balance < price {
        return Err(ApiError::BadRequest("Insufficient balance".into()));
    }

    // Debit
    sqlx::query("UPDATE users SET balance = balance - $1, is_pro = TRUE WHERE id = $2")
        .bind(price)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    // Deactivate existing sub
    sqlx::query(
        "UPDATE pro_subscriptions SET is_active = FALSE WHERE user_id = $1 AND is_active = TRUE",
    )
    .bind(auth.user_id)
    .execute(&mut *tx)
    .await?;

    // Create sub
    sqlx::query(
        "INSERT INTO pro_subscriptions (user_id, plan_type, period, amount_paid, expires_at) VALUES ($1, $2, $3, $4, NOW() + make_interval(days => $5))",
    )
    .bind(auth.user_id)
    .bind(req.plan_type)
    .bind(&req.period)
    .bind(price)
    .bind(duration_days as i32)
    .fetch_optional(&mut *tx)
    .await?;

    // Transaction record
    sqlx::query(
        "INSERT INTO payment_transactions (user_id, amount, currency, provider, type, status) VALUES ($1, $2, 'USD', 'wallet', 'pro_subscription', 'completed')",
    )
    .bind(auth.user_id)
    .bind(price)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(
        json!({ "data": { "subscribed": true, "plan_type": req.plan_type, "period": req.period } }),
    ))
}

/// POST /v1/payments/pro/refund — User requests a pro membership refund
pub async fn request_refund(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RefundRequest>,
) -> Result<Json<Value>, ApiError> {
    // Must be a pro user
    let is_pro: i16 = sqlx::query_scalar("SELECT COALESCE(is_pro, 0) FROM users WHERE id = $1")
        .bind(auth.user_id)
        .fetch_one(&state.db)
        .await?;

    if is_pro == 0 {
        return Err(ApiError::BadRequest("You are not a pro member".into()));
    }

    // Check no existing pending request
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM refund_requests WHERE user_id = $1 AND status = 0)",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if existing {
        return Err(ApiError::BadRequest(
            "You already have a pending refund request".into(),
        ));
    }

    let valid_types = ["star", "hot", "ultima", "vip"];
    if !valid_types.contains(&req.pro_type.as_str()) {
        return Err(ApiError::BadRequest("Invalid pro_type".into()));
    }

    // Get last payment hash
    let order_hash: String = sqlx::query_scalar(
        "SELECT COALESCE(provider_ref, id::text) FROM payment_transactions WHERE user_id = $1 AND type = 'pro_subscription' ORDER BY id DESC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .unwrap_or_else(|| "N/A".to_string());

    sqlx::query(
        "INSERT INTO refund_requests (user_id, order_hash_id, pro_type, description, status) VALUES ($1, $2, $3, $4, 0)",
    )
    .bind(auth.user_id)
    .bind(&order_hash)
    .bind(req.pro_type.trim())
    .bind(req.description.as_deref().unwrap_or(""))
    .execute(&state.db)
    .await?;

    Ok(Json(
        json!({ "data": { "submitted": true, "message": "Your refund request has been submitted. You will be notified once it is reviewed." } }),
    ))
}

/// POST /v1/payments/bank-receipt — Upload a bank transfer receipt
pub async fn upload_bank_receipt(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BankReceiptRequest>,
) -> Result<Json<Value>, ApiError> {
    if req.receipt_file.trim().is_empty() {
        return Err(ApiError::BadRequest("receipt_file is required".into()));
    }

    let id: i64 = sqlx::query_scalar(
        "INSERT INTO bank_receipts (user_id, description, price, mode, receipt_file) VALUES ($1, $2, $3, $4, $5) RETURNING id",
    )
    .bind(auth.user_id)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(req.price)
    .bind(req.mode.as_deref().unwrap_or("wallet"))
    .bind(req.receipt_file.trim())
    .fetch_one(&state.db)
    .await?;

    Ok(Json(
        json!({ "data": { "id": id, "message": "Bank receipt uploaded. An admin will review it shortly." } }),
    ))
}

pub async fn cancel(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let affected = sqlx::query(
        "UPDATE pro_subscriptions SET is_active = FALSE WHERE user_id = $1 AND is_active = TRUE",
    )
    .bind(auth.user_id)
    .execute(&state.db)
    .await?
    .rows_affected();

    if affected == 0 {
        return Err(ApiError::BadRequest("No active subscription found".into()));
    }

    sqlx::query("UPDATE users SET is_pro = FALSE WHERE id = $1")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({ "data": { "cancelled": true } })))
}
