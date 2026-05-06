-- posts.view_count was created as INT in 003; 018's ADD IF NOT EXISTS did not widen it.
ALTER TABLE posts
    ALTER COLUMN view_count TYPE BIGINT USING view_count::bigint;
