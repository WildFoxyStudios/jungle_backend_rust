-- Legacy migration 011 created `gifts` with `media_file` and `user_gifts.receiver_id`.
-- Commerce + messaging expect `image`, `price`, `is_active`, and `recipient_id`.

CREATE TABLE IF NOT EXISTS gift_categories (
    id BIGSERIAL PRIMARY KEY,
    name VARCHAR(200) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    sort_order INT DEFAULT 0
);

ALTER TABLE gifts ADD COLUMN IF NOT EXISTS category_id BIGINT REFERENCES gift_categories(id) ON DELETE SET NULL;
ALTER TABLE gifts ADD COLUMN IF NOT EXISTS image TEXT;
ALTER TABLE gifts ADD COLUMN IF NOT EXISTS price NUMERIC(10,2) DEFAULT 0;
ALTER TABLE gifts ADD COLUMN IF NOT EXISTS is_active BOOLEAN DEFAULT TRUE;

DO $$
BEGIN
  IF EXISTS (
      SELECT 1 FROM information_schema.columns
      WHERE table_schema = 'public' AND table_name = 'gifts' AND column_name = 'media_file'
  ) THEN
    UPDATE gifts
    SET image = media_file
    WHERE (image IS NULL OR TRIM(COALESCE(image, '')) = '')
      AND media_file IS NOT NULL
      AND TRIM(media_file) <> '';
  END IF;
END $$;

UPDATE gifts SET image = COALESCE(image, '') WHERE image IS NULL;
UPDATE gifts SET name = COALESCE(name, '') WHERE name IS NULL;
UPDATE gifts SET price = COALESCE(price, 0) WHERE price IS NULL;
UPDATE gifts SET is_active = COALESCE(is_active, TRUE) WHERE is_active IS NULL;

ALTER TABLE user_gifts ADD COLUMN IF NOT EXISTS message TEXT;

DO $$
BEGIN
  IF EXISTS (
      SELECT 1 FROM information_schema.columns
      WHERE table_schema = 'public' AND table_name = 'user_gifts' AND column_name = 'receiver_id'
  ) AND NOT EXISTS (
      SELECT 1 FROM information_schema.columns
      WHERE table_schema = 'public' AND table_name = 'user_gifts' AND column_name = 'recipient_id'
  ) THEN
    ALTER TABLE user_gifts RENAME COLUMN receiver_id TO recipient_id;
  END IF;
END $$;

ALTER TABLE user_gifts ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW();
