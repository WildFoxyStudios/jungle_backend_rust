#[derive(Debug, serde::Deserialize)]
pub struct OcrResult {
    pub extracted_text: String,
    pub confidence: f64,
}

/// Extract text from an image URL using OCR.space free API.
pub async fn extract_text(
    client: &reqwest::Client,
    api_key: &str,
    image_url: &str,
) -> Result<OcrResult, String> {
    let body = [
        ("apikey", api_key),
        ("url", image_url),
        ("language", "eng"),
        ("isOverlayRequired", "false"),
    ];

    let resp = client
        .post("https://api.ocr.space/parse/image")
        .form(&body)
        .send()
        .await
        .map_err(|e| format!("OCR request failed: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let parsed = &json["ParsedResults"][0];
    let text = parsed["ParsedText"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let confidence = parsed["TextOverlay"]["Lines"][0]["Words"][0]["Confidence"]
        .as_f64()
        .unwrap_or(0.0);

    Ok(OcrResult {
        extracted_text: text.trim().to_string(),
        confidence: (confidence * 100.0).round() / 100.0,
    })
}
