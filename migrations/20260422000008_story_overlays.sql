-- Migration: Story overlays (filter CSS + text style)
-- Plan §3.3 — S2 (text overlays), S3 (CSS filters).

ALTER TABLE story_media
    ADD COLUMN IF NOT EXISTS filter_css       TEXT,
    ADD COLUMN IF NOT EXISTS text_style_color VARCHAR(32),
    ADD COLUMN IF NOT EXISTS text_style_font  VARCHAR(128);

COMMENT ON COLUMN story_media.filter_css IS
    'CSS `filter` string applied by the viewer (e.g. "sepia(0.9)"). NULL = no filter.';

COMMENT ON COLUMN story_media.text_style_color IS
    'CSS color for the caption overlay. NULL = default (white).';

COMMENT ON COLUMN story_media.text_style_font IS
    'CSS font-family stack for the caption overlay. NULL = system default.';
