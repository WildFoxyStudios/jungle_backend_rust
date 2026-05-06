-- Live → VOD (Video-on-demand) HLS playback.
--
-- When a stream ends the publisher (HLS transcoder, Mux, Cloudflare
-- Stream, etc.) writes back the canonical playback URL plus optional
-- thumbnail and duration. Storing it on `live_streams` keeps the join
-- to ended live streams free for the LiveStreamPage's "Watch replay"
-- toggle and the `/v1/live/{id}/vod` endpoint.
--
-- Idempotent for repeat deploys.

ALTER TABLE live_streams
    ADD COLUMN IF NOT EXISTS vod_url TEXT,
    ADD COLUMN IF NOT EXISTS vod_thumbnail TEXT,
    ADD COLUMN IF NOT EXISTS vod_duration_seconds INT,
    ADD COLUMN IF NOT EXISTS vod_ready_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_live_streams_vod_ready
    ON live_streams(vod_ready_at)
    WHERE vod_ready_at IS NOT NULL;
