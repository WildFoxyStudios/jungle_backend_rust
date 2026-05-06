use axum::{
    Json,
    extract::{Path, Query, State},
};
use serde::Serialize;
use serde_json::{Value, json};
use shared::{
    auth::{AppState, AuthUser},
    errors::ApiError,
    pagination::PaginationParams,
};
use sqlx::FromRow;
use time::OffsetDateTime;

#[derive(Debug, Serialize, FromRow)]
pub struct GameRow {
    pub id: i64,
    pub name: String,
    pub avatar: String,
    pub link: String,
    pub active: bool,
    pub player_count: i32,
    pub created_at: OffsetDateTime,
}

pub async fn list_games(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Value>, ApiError> {
    let limit = params.limit();
    let cursor = params.cursor_id();

    let games = sqlx::query_as::<_, GameRow>(
        r#"
        SELECT * FROM games WHERE active = TRUE
          AND ($1::bigint IS NULL OR id < $1)
        ORDER BY player_count DESC, id DESC LIMIT $2
        "#,
    )
    .bind(cursor)
    .bind(limit + 1)
    .fetch_all(&state.db)
    .await?;

    let has_more = games.len() as i64 > limit;
    let games: Vec<_> = games.into_iter().take(limit as usize).collect();

    Ok(Json(
        json!({ "data": games, "meta": { "has_more": has_more } }),
    ))
}

/// GET /v1/games/{id} — get a single game by ID
pub async fn get_game(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let game = sqlx::query_as::<_, GameRow>("SELECT * FROM games WHERE id = $1 AND active = TRUE")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Game not found".into()))?;

    Ok(Json(json!({ "data": game })))
}

pub async fn play_game(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let game = sqlx::query_as::<_, GameRow>("SELECT * FROM games WHERE id = $1 AND active = TRUE")
        .bind(id)
        .fetch_optional(&state.db)
        .await?
        .ok_or_else(|| ApiError::NotFound("Game not found".into()))?;

    let result = sqlx::query(
        "INSERT INTO game_players (game_id, user_id) VALUES ($1, $2) ON CONFLICT (game_id, user_id) DO UPDATE SET last_played_at = NOW()",
    )
    .bind(id)
    .bind(auth.user_id)
    .execute(&state.db)
    .await?;

    if result.rows_affected() > 0 {
        sqlx::query("UPDATE games SET player_count = (SELECT COUNT(*) FROM game_players WHERE game_id = $1) WHERE id = $1")
            .bind(id)
            .execute(&state.db)
            .await?;
    }

    Ok(Json(json!({ "data": { "link": game.link } })))
}

pub async fn my_games(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let games = sqlx::query_as::<_, GameRow>(
        r#"
        SELECT g.* FROM games g
        JOIN game_players gp ON gp.game_id = g.id
        WHERE gp.user_id = $1 AND g.active = TRUE
        ORDER BY gp.last_played_at DESC
        LIMIT 50
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(json!({ "data": games })))
}
