/// Transcribe audio using Cloudflare Workers AI Whisper model.
/// audio_url should be a publicly accessible URL to the audio file.
pub async fn transcribe(
    client: &reqwest::Client,
    account_id: &str,
    api_token: &str,
    audio_url: &str,
) -> Result<String, String> {
    let model = "@cf/openai/whisper";
    let url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/ai/run/{}",
        account_id, model
    );

    let body = serde_json::json!({
        "audio_url": audio_url,
    });

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_token))
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Cloudflare AI request failed: {}", e))?;

    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if json["success"].as_bool() == Some(true) {
        Ok(json["result"]["text"].as_str().unwrap_or("").to_string())
    } else {
        let errors = json["errors"].to_string();
        Err(format!("Cloudflare AI error: {}", errors))
    }
}
