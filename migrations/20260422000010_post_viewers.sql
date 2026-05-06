-- Migration: Track individual post viewers (not just a counter)
-- Plan §3.18 — SA7: "Views info modal" listing who viewed a post.
-- Frontend `ViewersLightbox.tsx` queries `GET /v1/posts/{id}/viewers`.

CREATE TABLE IF NOT EXISTS post_viewers (
    id         BIGSERIAL PRIMARY KEY,
    post_id    BIGINT NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
    user_id    BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    viewed_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(post_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_post_viewers_post
    ON post_viewers(post_id, viewed_at DESC);
CREATE INDEX IF NOT EXISTS idx_post_viewers_user
    ON post_viewers(user_id, viewed_at DESC);

COMMENT ON TABLE post_viewers IS
    'Audit trail of who viewed which post. Persists at most one row per (post,user); repeated views update `viewed_at` via ON CONFLICT. The `posts.post_views` counter is still used for fast display.';
