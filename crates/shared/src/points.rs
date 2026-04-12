use redis::AsyncCommands;
use sqlx::PgPool;

use crate::errors::ApiError;

#[derive(Debug, Clone)]
pub struct PointsConfig {
    pub enabled: bool,
    pub comments_point: i64,
    pub likes_point: i64,
    pub dislikes_point: i64,
    pub wonders_point: i64,
    pub reaction_point: i64,
    pub create_post_point: i64,
    pub create_blog_point: i64,
    pub admob_point: i64,
    pub dollar_to_point_cost: f64,
    pub allow_withdrawal: bool,
    pub free_day_limit: i64,
    pub pro_day_limit: i64,
}

impl Default for PointsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            comments_point: 1,
            likes_point: 1,
            dislikes_point: 1,
            wonders_point: 2,
            reaction_point: 1,
            create_post_point: 5,
            create_blog_point: 10,
            admob_point: 1,
            dollar_to_point_cost: 1000.0,
            allow_withdrawal: false,
            free_day_limit: 50,
            pro_day_limit: 200,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PointAction {
    Comment,
    Like,
    Dislike,
    Wonder,
    Reaction,
    CreatePost,
    CreateBlog,
    Admob,
}

pub async fn register_points(
    db: &PgPool,
    redis: &mut redis::aio::ConnectionManager,
    config: &PointsConfig,
    user_id: i64,
    action: PointAction,
    is_pro: bool,
) -> Result<(), ApiError> {
    if !config.enabled {
        return Ok(());
    }

    let points = match action {
        PointAction::Comment => config.comments_point,
        PointAction::Like => config.likes_point,
        PointAction::Dislike => config.dislikes_point,
        PointAction::Wonder => config.wonders_point,
        PointAction::Reaction => config.reaction_point,
        PointAction::CreatePost => config.create_post_point,
        PointAction::CreateBlog => config.create_blog_point,
        PointAction::Admob => config.admob_point,
    };

    if points == 0 {
        return Ok(());
    }

    // Daily limit check via Redis
    let today = time::OffsetDateTime::now_utc();
    let daily_key = format!(
        "daily_points:{}:{}-{:02}-{:02}",
        user_id,
        today.year(),
        today.month() as u8,
        today.day()
    );
    let current: i64 = redis.get(&daily_key).await.unwrap_or(0);
    let limit = if is_pro {
        config.pro_day_limit
    } else {
        config.free_day_limit
    };

    if current + points > limit {
        return Ok(()); // Silently cap
    }

    // Update user points
    let wallet_delta = points as f64 / config.dollar_to_point_cost;
    if config.allow_withdrawal {
        sqlx::query(
            "UPDATE users SET points = COALESCE(points, 0) + $1, balance = GREATEST(COALESCE(balance, 0) + $2, 0) WHERE id = $3",
        )
        .bind(points)
        .bind(wallet_delta)
        .bind(user_id)
        .execute(db)
        .await?;
    } else {
        sqlx::query(
            "UPDATE users SET points = COALESCE(points, 0) + $1, wallet = GREATEST(COALESCE(wallet, 0) + $2, 0) WHERE id = $3",
        )
        .bind(points)
        .bind(wallet_delta)
        .bind(user_id)
        .execute(db)
        .await?;
    }

    // Increment daily counter
    let _: Result<i64, _> = redis.incr(&daily_key, points).await;
    let _: Result<(), _> = redis.expire(&daily_key, 86400).await;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_points_config_defaults() {
        let cfg = PointsConfig::default();
        assert!(!cfg.enabled);
        assert_eq!(cfg.comments_point, 1);
        assert_eq!(cfg.create_post_point, 5);
        assert_eq!(cfg.create_blog_point, 10);
        assert_eq!(cfg.free_day_limit, 50);
        assert_eq!(cfg.pro_day_limit, 200);
        assert!(!cfg.allow_withdrawal);
    }

    #[test]
    fn test_point_action_values() {
        let cfg = PointsConfig::default();
        let cases = vec![
            (PointAction::Comment, cfg.comments_point),
            (PointAction::Like, cfg.likes_point),
            (PointAction::Dislike, cfg.dislikes_point),
            (PointAction::Wonder, cfg.wonders_point),
            (PointAction::Reaction, cfg.reaction_point),
            (PointAction::CreatePost, cfg.create_post_point),
            (PointAction::CreateBlog, cfg.create_blog_point),
            (PointAction::Admob, cfg.admob_point),
        ];
        for (action, expected) in cases {
            let points = match action {
                PointAction::Comment => cfg.comments_point,
                PointAction::Like => cfg.likes_point,
                PointAction::Dislike => cfg.dislikes_point,
                PointAction::Wonder => cfg.wonders_point,
                PointAction::Reaction => cfg.reaction_point,
                PointAction::CreatePost => cfg.create_post_point,
                PointAction::CreateBlog => cfg.create_blog_point,
                PointAction::Admob => cfg.admob_point,
            };
            assert_eq!(points, expected);
        }
    }

    #[test]
    fn test_dollar_to_point_cost() {
        let cfg = PointsConfig::default();
        let wallet_delta = 5_f64 / cfg.dollar_to_point_cost;
        assert!((wallet_delta - 0.005).abs() < f64::EPSILON);
    }
}
