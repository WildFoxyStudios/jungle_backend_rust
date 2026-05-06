//! Order-invoice PDF generator.
//!
//! Ports the PHP `api/market` action `download_invoice` — builds a printable
//! invoice PDF for a finished order and streams it back as an
//! `application/pdf` response. Uses `lopdf` 0.40 with the PDF base-14
//! Helvetica font, so no external font file needs to be bundled.

use axum::{
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::Response,
};
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, Stream, dictionary};
use rust_decimal::Decimal;
use serde_json::Value;
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
};
use sqlx::FromRow;
use time::OffsetDateTime;

// ─── DB row types ────────────────────────────────────────────────────────────

#[derive(Debug, FromRow)]
struct InvoiceRow {
    order_id: i64,
    order_uuid: uuid::Uuid,
    buyer_id: i64,
    seller_id: i64,
    quantity: i32,
    total_price: Decimal,
    status: String,
    address: Option<Value>,
    created_at: OffsetDateTime,
    product_name: String,
    product_price: Decimal,
    buyer_username: String,
    buyer_first_name: String,
    buyer_last_name: String,
    buyer_email: String,
    seller_username: String,
    seller_first_name: String,
    seller_last_name: String,
    seller_email: String,
}

// ─── Handler ─────────────────────────────────────────────────────────────────

/// GET /v1/orders/{id}/invoice — Download a printable PDF invoice for a finished order.
pub async fn download_invoice(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Response, ApiError> {
    let row = sqlx::query_as::<_, InvoiceRow>(
        r#"
        SELECT o.id            AS order_id,
               o.uuid          AS order_uuid,
               o.buyer_id      AS buyer_id,
               o.seller_id     AS seller_id,
               o.quantity      AS quantity,
               o.total_price   AS total_price,
               o.status        AS status,
               o.address       AS address,
               o.created_at    AS created_at,
               p.name          AS product_name,
               p.price         AS product_price,
               bu.username     AS buyer_username,
               bu.first_name   AS buyer_first_name,
               bu.last_name    AS buyer_last_name,
               bu.email        AS buyer_email,
               su.username     AS seller_username,
               su.first_name   AS seller_first_name,
               su.last_name    AS seller_last_name,
               su.email        AS seller_email
          FROM orders o
          JOIN products p   ON p.id = o.product_id
          JOIN users   bu  ON bu.id = o.buyer_id
          JOIN users   su  ON su.id = o.seller_id
         WHERE o.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await?
    .ok_or_else(|| ApiError::NotFound("Order not found".into()))?;

    if row.buyer_id != auth.user_id && row.seller_id != auth.user_id && !auth.is_admin {
        return Err(ApiError::Forbidden("".into()));
    }

    let site_name = std::env::var("SITE_NAME").unwrap_or_else(|_| "WoWonder".into());
    let currency = std::env::var("CURRENCY").unwrap_or_else(|_| "USD".into());

    let pdf_bytes = build_invoice_pdf(&row, &site_name, &currency)
        .map_err(|e| ApiError::Internal(format!("Failed to build invoice PDF: {}", e)))?;

    let filename = format!("invoice-{}.pdf", row.order_uuid);
    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/pdf"),
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{}\"", filename))
            .unwrap_or_else(|_| HeaderValue::from_static("attachment; filename=\"invoice.pdf\"")),
    );
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );

    let mut response = Response::new(Body::from(pdf_bytes));
    *response.status_mut() = StatusCode::OK;
    *response.headers_mut() = headers;
    Ok(response)
}

// ─── PDF builder ─────────────────────────────────────────────────────────────

/// A4 page height in PDF user-space units (1/72 in, 595 × 842).
const PAGE_W: i64 = 595;
const PAGE_H: i64 = 842;

fn build_invoice_pdf(
    row: &InvoiceRow,
    site_name: &str,
    currency: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut doc = Document::with_version("1.5");

    let pages_id = doc.new_object_id();
    let regular_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
        "Encoding" => "WinAnsiEncoding",
    });
    let bold_font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica-Bold",
        "Encoding" => "WinAnsiEncoding",
    });

    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => regular_font_id,
            "F2" => bold_font_id,
        },
    });

    // Build a single page of content with a simple vertical layout.
    let mut ops: Vec<Operation> = Vec::new();
    let mut cursor_y = PAGE_H - 60;

    // Header: site name (right-aligned visual) + INVOICE title
    write_text(&mut ops, "F2", 22, 50, cursor_y, site_name);
    write_text(&mut ops, "F2", 22, 430, cursor_y, "INVOICE");
    cursor_y -= 30;

    // Divider
    ops.push(Operation::new("q", vec![]));
    ops.push(Operation::new("0.6 0.6 0.6 RG", vec![]));
    ops.push(Operation::new("0.5 w", vec![]));
    ops.push(Operation::new(
        "re",
        vec![50.into(), cursor_y.into(), (PAGE_W - 100).into(), 1.into()],
    ));
    ops.push(Operation::new("S", vec![]));
    ops.push(Operation::new("Q", vec![]));
    cursor_y -= 30;

    // Invoice metadata block
    let date_str = row
        .created_at
        .format(&time::format_description::well_known::Iso8601::DATE)
        .unwrap_or_else(|_| row.created_at.to_string());
    write_text(&mut ops, "F2", 10, 50, cursor_y, "Invoice #");
    write_text(
        &mut ops,
        "F1",
        10,
        115,
        cursor_y,
        &row.order_uuid.to_string(),
    );
    write_text(&mut ops, "F2", 10, 400, cursor_y, "Date");
    write_text(&mut ops, "F1", 10, 435, cursor_y, &date_str);
    cursor_y -= 16;

    write_text(&mut ops, "F2", 10, 50, cursor_y, "Status");
    write_text(&mut ops, "F1", 10, 115, cursor_y, &row.status);
    write_text(&mut ops, "F2", 10, 400, cursor_y, "Order ID");
    write_text(&mut ops, "F1", 10, 435, cursor_y, &row.order_id.to_string());
    cursor_y -= 28;

    // Parties: Seller (left) — Buyer (right)
    write_text(&mut ops, "F2", 11, 50, cursor_y, "Seller");
    write_text(&mut ops, "F2", 11, 320, cursor_y, "Buyer");
    cursor_y -= 16;

    let seller_name = display_name(
        &row.seller_first_name,
        &row.seller_last_name,
        &row.seller_username,
    );
    let buyer_name = display_name(
        &row.buyer_first_name,
        &row.buyer_last_name,
        &row.buyer_username,
    );

    write_text(&mut ops, "F1", 10, 50, cursor_y, &seller_name);
    write_text(&mut ops, "F1", 10, 320, cursor_y, &buyer_name);
    cursor_y -= 14;
    write_text(&mut ops, "F1", 10, 50, cursor_y, &row.seller_email);
    write_text(&mut ops, "F1", 10, 320, cursor_y, &row.buyer_email);
    cursor_y -= 14;

    // Shipping address (buyer side only, if present)
    if let Some(addr) = &row.address {
        let addr_text = format_address(addr);
        for line in addr_text.lines().take(4) {
            write_text(&mut ops, "F1", 10, 320, cursor_y, line);
            cursor_y -= 12;
        }
    }
    cursor_y -= 14;

    // Items table header
    write_text(&mut ops, "F2", 11, 50, cursor_y, "Description");
    write_text(&mut ops, "F2", 11, 350, cursor_y, "Qty");
    write_text(&mut ops, "F2", 11, 410, cursor_y, "Unit price");
    write_text(&mut ops, "F2", 11, 510, cursor_y, "Total");
    cursor_y -= 12;

    ops.push(Operation::new("q", vec![]));
    ops.push(Operation::new("0.6 0.6 0.6 RG", vec![]));
    ops.push(Operation::new("0.5 w", vec![]));
    ops.push(Operation::new(
        "re",
        vec![50.into(), cursor_y.into(), (PAGE_W - 100).into(), 1.into()],
    ));
    ops.push(Operation::new("S", vec![]));
    ops.push(Operation::new("Q", vec![]));
    cursor_y -= 18;

    // Items (single-product order — matches current schema)
    let product_label = truncate(&row.product_name, 55);
    write_text(&mut ops, "F1", 10, 50, cursor_y, &product_label);
    write_text(&mut ops, "F1", 10, 350, cursor_y, &row.quantity.to_string());
    write_text(
        &mut ops,
        "F1",
        10,
        410,
        cursor_y,
        &format!("{} {}", currency, fmt_decimal(&row.product_price)),
    );
    write_text(
        &mut ops,
        "F1",
        10,
        510,
        cursor_y,
        &format!("{} {}", currency, fmt_decimal(&row.total_price)),
    );
    cursor_y -= 30;

    // Total
    write_text(&mut ops, "F2", 12, 380, cursor_y, "TOTAL");
    write_text(
        &mut ops,
        "F2",
        12,
        460,
        cursor_y,
        &format!("{} {}", currency, fmt_decimal(&row.total_price)),
    );
    cursor_y -= 50;

    // Footer / thank you
    write_text(
        &mut ops,
        "F1",
        9,
        50,
        cursor_y,
        "Thank you for your purchase.",
    );
    cursor_y -= 11;
    write_text(
        &mut ops,
        "F1",
        9,
        50,
        cursor_y,
        &format!(
            "Generated by {} — this is a computer-generated document.",
            site_name
        ),
    );

    let content = Content { operations: ops };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode()?));

    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
    });

    let pages_dict = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), PAGE_W.into(), PAGE_H.into()],
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages_dict));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.compress();

    let mut out: Vec<u8> = Vec::with_capacity(4096);
    doc.save_to(&mut out)?;
    Ok(out)
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

/// Emit a `BT ... ET` block that draws a single text line at `(x, y)`.
fn write_text(ops: &mut Vec<Operation>, font: &str, size: i64, x: i64, y: i64, text: &str) {
    ops.push(Operation::new("BT", vec![]));
    ops.push(Operation::new("Tf", vec![font.into(), size.into()]));
    ops.push(Operation::new("Td", vec![x.into(), y.into()]));
    ops.push(Operation::new(
        "Tj",
        vec![Object::string_literal(encode_win_ansi(text))],
    ));
    ops.push(Operation::new("ET", vec![]));
}

/// Encode text into WinAnsiEncoding (CP1252) bytes. Characters not in the
/// encoding are replaced by '?'. This is enough for the Latin-1 invoice data
/// we generate (English + accents); emoji/CJK fall back to '?'.
fn encode_win_ansi(input: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for c in input.chars() {
        let byte = match c as u32 {
            // ASCII passes through
            0x20..=0x7E => c as u8,
            // WinAnsi extras (CP1252)
            0x2022 => 0x95,             // bullet
            0x2013 => 0x96,             // en dash
            0x2014 => 0x97,             // em dash
            0x2018 => 0x91,             // left single quote
            0x2019 => 0x92,             // right single quote
            0x201C => 0x93,             // left double quote
            0x201D => 0x94,             // right double quote
            0x00A0..=0x00FF => c as u8, // Latin-1 supplement
            0x0152 => 0x8C,             // OE
            0x0153 => 0x9C,             // oe
            0x0160 => 0x8A,             // Š
            0x0161 => 0x9A,             // š
            0x0178 => 0x9F,             // Ÿ
            0x017D => 0x8E,             // Ž
            0x017E => 0x9E,             // ž
            _ => b'?',
        };
        out.push(byte);
    }
    out
}

fn display_name(first: &str, last: &str, username: &str) -> String {
    let full = format!("{} {}", first.trim(), last.trim())
        .trim()
        .to_string();
    if full.is_empty() {
        format!("@{}", username)
    } else {
        full
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{}…", truncated)
    }
}

fn fmt_decimal(d: &Decimal) -> String {
    // Always show two decimals — friendlier for money.
    format!("{:.2}", d.round_dp(2))
}

fn format_address(v: &Value) -> String {
    // The `address` column is JSONB and free-form in WoWonder. Try the common
    // fields first, then fall back to a flattened string.
    if let Some(obj) = v.as_object() {
        let mut lines: Vec<String> = Vec::new();

        // Recipient name (if provided) goes on its own line.
        if let Some(val) = obj.get("name").and_then(|x| x.as_str())
            && !val.trim().is_empty()
        {
            lines.push(val.trim().to_string());
        }
        // Street line 1 — try aliases in order of preference.
        for key in ["line1", "line_1", "street", "address"] {
            if let Some(val) = obj.get(key).and_then(|x| x.as_str())
                && !val.trim().is_empty()
            {
                lines.push(val.trim().to_string());
                break;
            }
        }
        // Optional street line 2.
        for key in ["line2", "line_2", "address2"] {
            if let Some(val) = obj.get(key).and_then(|x| x.as_str())
                && !val.trim().is_empty()
            {
                lines.push(val.trim().to_string());
                break;
            }
        }
        let city = obj.get("city").and_then(|x| x.as_str()).unwrap_or("");
        let state = obj.get("state").and_then(|x| x.as_str()).unwrap_or("");
        let zip = obj
            .get("zip")
            .or_else(|| obj.get("postal_code"))
            .or_else(|| obj.get("zipcode"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let city_line = [city, state, zip]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(", ");
        if !city_line.is_empty() {
            lines.push(city_line);
        }
        if let Some(country) = obj.get("country").and_then(|x| x.as_str())
            && !country.trim().is_empty()
        {
            lines.push(country.to_string());
        }
        if !lines.is_empty() {
            return lines.join("\n");
        }
    }
    v.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_keeps_short_strings() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_shortens_long_strings() {
        let out = truncate("123456789012345", 10);
        assert_eq!(out.chars().count(), 10);
        assert!(out.ends_with('…'));
    }

    #[test]
    fn fmt_decimal_two_places() {
        assert_eq!(fmt_decimal(&Decimal::new(1000, 2)), "10.00");
        // `round_dp` uses banker's rounding (half-to-even), so 12.345 → 12.34.
        assert_eq!(fmt_decimal(&Decimal::new(12345, 3)), "12.34");
        // Non-tie rounds normally.
        assert_eq!(fmt_decimal(&Decimal::new(12346, 3)), "12.35");
    }

    #[test]
    fn encode_win_ansi_ascii_passthrough() {
        assert_eq!(encode_win_ansi("abc"), b"abc");
    }

    #[test]
    fn encode_win_ansi_replaces_non_latin() {
        // Emoji → '?'
        let out = encode_win_ansi("a🙂b");
        assert_eq!(out, b"a?b");
    }

    #[test]
    fn display_name_prefers_first_last() {
        assert_eq!(display_name("John", "Doe", "johndoe"), "John Doe");
    }

    #[test]
    fn display_name_falls_back_to_username() {
        assert_eq!(display_name("", "", "johndoe"), "@johndoe");
    }

    #[test]
    fn format_address_extracts_common_fields() {
        let v: Value = serde_json::json!({
            "name": "Jane Doe",
            "line1": "123 Main St",
            "city": "Springfield",
            "state": "IL",
            "zip": "62704",
            "country": "USA"
        });
        let formatted = format_address(&v);
        assert!(formatted.contains("123 Main St"));
        assert!(formatted.contains("Springfield, IL, 62704"));
        assert!(formatted.contains("USA"));
    }
}
