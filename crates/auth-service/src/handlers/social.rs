use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use shared::{auth::AppState, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct SocialLoginRequest {
    pub provider: String,
    pub access_token: String,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct SocialUser {
    pub id: String,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    pub avatar: Option<String>,
}

pub async fn social_login(
    State(state): State<AppState>,
    Json(req): Json<SocialLoginRequest>,
) -> Result<Json<Value>, ApiError> {
    let social_user = fetch_social_user(&req.provider, &req.access_token).await?;

    // Try to find existing user by social login
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM users WHERE social_logins @> $1::jsonb AND deleted_at IS NULL",
    )
    .bind(json!({ &req.provider: { "id": &social_user.id } }))
    .fetch_optional(&state.db)
    .await?;

    let user_id = if let Some(id) = existing {
        id
    } else if !social_user.email.is_empty() {
        // Try to find by email
        let by_email = sqlx::query_scalar::<_, i64>(
            "SELECT id FROM users WHERE email = $1 AND deleted_at IS NULL",
        )
        .bind(&social_user.email)
        .fetch_optional(&state.db)
        .await?;

        if let Some(id) = by_email {
            // Link social account to existing user
            sqlx::query(
                "UPDATE users SET social_logins = COALESCE(social_logins, '{}'::jsonb) || $1::jsonb WHERE id = $2",
            )
            .bind(json!({ &req.provider: { "id": &social_user.id } }))
            .bind(id)
            .execute(&state.db)
            .await?;
            id
        } else {
            // Auto-register new user
            create_social_user(&state, &req.provider, &social_user).await?
        }
    } else {
        create_social_user(&state, &req.provider, &social_user).await?
    };

    // Generate tokens (same as regular login)
    let access_exp = time::OffsetDateTime::now_utc() + time::Duration::minutes(15);
    let refresh_exp = time::OffsetDateTime::now_utc() + time::Duration::days(30);

    let is_admin = sqlx::query_scalar::<_, bool>(
        "SELECT COALESCE(is_admin, FALSE) FROM users WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .unwrap_or(false);

    let claims = shared::auth::Claims {
        sub: user_id,
        uuid: uuid::Uuid::new_v4(),
        is_admin,
        exp: access_exp.unix_timestamp(),
        iat: time::OffsetDateTime::now_utc().unix_timestamp(),
    };

    let access_token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(|e| ApiError::Internal(e.to_string()))?;

    let refresh_token = uuid::Uuid::new_v4().to_string();
    let token_hash = format!("{:x}", sha2::Sha256::digest(refresh_token.as_bytes()));

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, platform, expires_at) VALUES ($1, $2, $3, $4)",
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(&req.provider)
    .bind(refresh_exp)
    .execute(&state.db)
    .await?;

    // Update last login
    sqlx::query("UPDATE users SET last_active = NOW() WHERE id = $1")
        .bind(user_id)
        .execute(&state.db)
        .await?;

    Ok(Json(json!({
        "data": {
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": 900,
            "user_id": user_id
        }
    })))
}

async fn create_social_user(
    state: &AppState,
    provider: &str,
    social: &SocialUser,
) -> Result<i64, ApiError> {
    let username = generate_unique_username(state, &social.first_name).await?;

    let user_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO users (username, email, first_name, last_name, avatar, social_logins, email_verified, is_active)
        VALUES ($1, $2, $3, $4, $5, $6::jsonb, TRUE, TRUE)
        RETURNING id
        "#,
    )
    .bind(&username)
    .bind(if social.email.is_empty() {
        format!("{}@{}.social", social.id, provider)
    } else {
        social.email.clone()
    })
    .bind(&social.first_name)
    .bind(&social.last_name)
    .bind(&social.avatar)
    .bind(json!({ provider: { "id": &social.id } }))
    .fetch_one(&state.db)
    .await?;

    Ok(user_id)
}

async fn generate_unique_username(state: &AppState, name: &str) -> Result<String, ApiError> {
    let base = name
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .take(20)
        .collect::<String>();
    let base = if base.is_empty() {
        "user".to_string()
    } else {
        base
    };

    for i in 0..100 {
        let candidate = if i == 0 {
            base.clone()
        } else {
            format!("{}_{}", base, i)
        };

        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE username = $1)",
        )
        .bind(&candidate)
        .fetch_one(&state.db)
        .await?;

        if !exists {
            return Ok(candidate);
        }
    }

    Ok(format!("{}_{}", base, uuid::Uuid::new_v4().simple()))
}

// ── Provider implementations ──

async fn fetch_social_user(provider: &str, token: &str) -> Result<SocialUser, ApiError> {
    let client = reqwest::Client::new();

    match provider {
        // ── Core providers ──
        "google" => {
            let resp: Value = reqwest::get(format!(
                "https://oauth2.googleapis.com/tokeninfo?id_token={}",
                token
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["sub"].as_str().unwrap_or_default().to_string(),
                email: resp["email"].as_str().unwrap_or_default().to_string(),
                first_name: resp["given_name"].as_str().unwrap_or_default().to_string(),
                last_name: resp["family_name"].as_str().unwrap_or_default().to_string(),
                avatar: resp["picture"].as_str().map(String::from),
            })
        }

        "facebook" => {
            let resp: Value = reqwest::get(format!(
                "https://graph.facebook.com/me?fields=id,name,email,first_name,last_name,picture.type(large)&access_token={}",
                token
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["id"].as_str().unwrap_or_default().to_string(),
                email: resp["email"].as_str().unwrap_or_default().to_string(),
                first_name: resp["first_name"].as_str().unwrap_or_default().to_string(),
                last_name: resp["last_name"].as_str().unwrap_or_default().to_string(),
                avatar: resp["picture"]["data"]["url"].as_str().map(String::from),
            })
        }

        "twitter" => {
            let resp: Value = client
                .get("https://api.twitter.com/2/users/me?user.fields=profile_image_url,name")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .json()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            let data = &resp["data"];
            Ok(SocialUser {
                id: data["id"].as_str().unwrap_or_default().to_string(),
                email: String::new(),
                first_name: data["name"].as_str().unwrap_or_default().to_string(),
                last_name: String::new(),
                avatar: data["profile_image_url"].as_str().map(String::from),
            })
        }

        "apple" => {
            // Apple Sign In: verify JWT signature with Apple's public keys (JWKS)
            let claims = verify_apple_identity_token(token).await?;

            Ok(SocialUser {
                id: claims["sub"].as_str().unwrap_or_default().to_string(),
                email: claims["email"].as_str().unwrap_or_default().to_string(),
                first_name: String::new(),
                last_name: String::new(),
                avatar: None,
            })
        }

        "linkedin" => {
            let resp: Value = client
                .get("https://api.linkedin.com/v2/userinfo")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .json()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["sub"].as_str().unwrap_or_default().to_string(),
                email: resp["email"].as_str().unwrap_or_default().to_string(),
                first_name: resp["given_name"].as_str().unwrap_or_default().to_string(),
                last_name: resp["family_name"].as_str().unwrap_or_default().to_string(),
                avatar: resp["picture"].as_str().map(String::from),
            })
        }

        "discord" => {
            let resp: Value = client
                .get("https://discord.com/api/v10/users/@me")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .json()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            let id = resp["id"].as_str().unwrap_or_default().to_string();
            let avatar = resp["avatar"]
                .as_str()
                .map(|a| format!("https://cdn.discordapp.com/avatars/{}/{}.png", id, a));

            Ok(SocialUser {
                id,
                email: resp["email"].as_str().unwrap_or_default().to_string(),
                first_name: resp["global_name"]
                    .as_str()
                    .or(resp["username"].as_str())
                    .unwrap_or_default()
                    .to_string(),
                last_name: String::new(),
                avatar,
            })
        }

        "tiktok" => {
            let resp: Value = client
                .get("https://open.tiktokapis.com/v2/user/info/?fields=open_id,display_name,avatar_url")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .json()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            let user = &resp["data"]["user"];
            Ok(SocialUser {
                id: user["open_id"].as_str().unwrap_or_default().to_string(),
                email: String::new(),
                first_name: user["display_name"].as_str().unwrap_or_default().to_string(),
                last_name: String::new(),
                avatar: user["avatar_url"].as_str().map(String::from),
            })
        }

        "instagram" => {
            let resp: Value = reqwest::get(format!(
                "https://graph.instagram.com/me?fields=id,username&access_token={}",
                token
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["id"].as_str().unwrap_or_default().to_string(),
                email: String::new(),
                first_name: resp["username"].as_str().unwrap_or_default().to_string(),
                last_name: String::new(),
                avatar: None,
            })
        }

        // ── Regional providers ──
        "vkontakte" => {
            let resp: Value = reqwest::get(format!(
                "https://api.vk.com/method/users.get?access_token={}&fields=photo_200,screen_name&v=5.131",
                token
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            let u = &resp["response"][0];
            Ok(SocialUser {
                id: u["id"].to_string(),
                email: String::new(),
                first_name: u["first_name"].as_str().unwrap_or_default().to_string(),
                last_name: u["last_name"].as_str().unwrap_or_default().to_string(),
                avatar: u["photo_200"].as_str().map(String::from),
            })
        }

        "qq" => {
            // QQ OAuth: token = access_token, need to get openid first
            let openid_resp: Value = reqwest::get(format!(
                "https://graph.qq.com/oauth2.0/me?access_token={}",
                token
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .json()
            .await
            .unwrap_or_default();

            let openid = openid_resp["openid"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            Ok(SocialUser {
                id: openid,
                email: String::new(),
                first_name: "QQ User".to_string(),
                last_name: String::new(),
                avatar: None,
            })
        }

        "wechat" => {
            let resp: Value = reqwest::get(format!(
                "https://api.weixin.qq.com/sns/userinfo?access_token={}&openid=me",
                token
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["openid"].as_str().unwrap_or_default().to_string(),
                email: String::new(),
                first_name: resp["nickname"].as_str().unwrap_or_default().to_string(),
                last_name: String::new(),
                avatar: resp["headimgurl"].as_str().map(String::from),
            })
        }

        "mailru" => {
            let resp: Value = reqwest::get(format!(
                "https://oauth.mail.ru/userinfo?access_token={}",
                token
            ))
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?
            .json()
            .await
            .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["id"].as_str().unwrap_or_default().to_string(),
                email: resp["email"].as_str().unwrap_or_default().to_string(),
                first_name: resp["first_name"].as_str().unwrap_or_default().to_string(),
                last_name: resp["last_name"].as_str().unwrap_or_default().to_string(),
                avatar: resp["image"].as_str().map(String::from),
            })
        }

        "okru" => {
            // Odnoklassniki OAuth
            let resp: Value = client
                .get("https://api.ok.ru/fb.do?method=users.getCurrentUser&format=json")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .json()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["uid"].as_str().unwrap_or_default().to_string(),
                email: resp["email"].as_str().unwrap_or_default().to_string(),
                first_name: resp["first_name"].as_str().unwrap_or_default().to_string(),
                last_name: resp["last_name"].as_str().unwrap_or_default().to_string(),
                avatar: resp["pic_full"].as_str().map(String::from),
            })
        }

        "wordpress" => {
            let resp: Value = client
                .get("https://public-api.wordpress.com/rest/v1.1/me")
                .bearer_auth(token)
                .send()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?
                .json()
                .await
                .map_err(|e| ApiError::Internal(e.to_string()))?;

            Ok(SocialUser {
                id: resp["ID"].to_string(),
                email: resp["email"].as_str().unwrap_or_default().to_string(),
                first_name: resp["display_name"].as_str().unwrap_or_default().to_string(),
                last_name: String::new(),
                avatar: resp["avatar_URL"].as_str().map(String::from),
            })
        }

        _ => Err(ApiError::BadRequest(format!(
            "Unsupported provider: {}",
            provider
        ))),
    }
}

// ── Apple JWKS verification ──

#[derive(Debug, Deserialize)]
struct AppleJwks {
    keys: Vec<AppleJwk>,
}

#[derive(Debug, Deserialize)]
struct AppleJwk {
    kty: String,
    kid: String,
    #[serde(rename = "use")]
    key_use: Option<String>,
    alg: Option<String>,
    n: String,
    e: String,
}

async fn verify_apple_identity_token(token: &str) -> Result<Value, ApiError> {
    use base64::Engine;
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};

    // 1. Decode header to get "kid"
    let header = jsonwebtoken::decode_header(token)
        .map_err(|e| ApiError::BadRequest(format!("Invalid Apple token header: {}", e)))?;
    let kid = header.kid.ok_or_else(|| ApiError::BadRequest("Apple token missing kid".into()))?;

    // 2. Fetch Apple's public keys
    let jwks: AppleJwks = reqwest::get("https://appleid.apple.com/auth/keys")
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to fetch Apple JWKS: {}", e)))?
        .json()
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to parse Apple JWKS: {}", e)))?;

    // 3. Find matching key by kid
    let jwk = jwks
        .keys
        .iter()
        .find(|k| {
            k.kid == kid
                && k.kty == "RSA"
                && k.key_use.as_deref() != Some("enc") // exclude encryption-only keys
                && k.alg.as_deref().unwrap_or("RS256") == "RS256"
        })
        .ok_or_else(|| ApiError::BadRequest("Apple JWKS key not found for kid".into()))?;

    // 4. Build RSA public key from modulus (n) and exponent (e)
    let n_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&jwk.n)
        .map_err(|_| ApiError::BadRequest("Invalid Apple JWK modulus".into()))?;
    let e_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&jwk.e)
        .map_err(|_| ApiError::BadRequest("Invalid Apple JWK exponent".into()))?;

    let decoding_key = DecodingKey::from_rsa_raw_components(&n_bytes, &e_bytes);

    // 5. Validate JWT (issuer, audience, expiry, signature)
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&["https://appleid.apple.com"]);
    let apple_client_id = std::env::var("APPLE_CLIENT_ID").unwrap_or_default();
    if !apple_client_id.is_empty() {
        validation.set_audience(&[&apple_client_id]);
    } else {
        validation.validate_aud = false;
    }

    let token_data = decode::<Value>(token, &decoding_key, &validation)
        .map_err(|e| ApiError::BadRequest(format!("Apple token verification failed: {}", e)))?;

    Ok(token_data.claims)
}
