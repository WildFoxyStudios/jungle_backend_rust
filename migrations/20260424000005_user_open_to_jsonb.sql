-- WoWonder's "open to" (relationship/work/etc) field originally stored a
-- comma-separated string. The Rust services already serialize the value
-- as JSON when writing, so converting the underlying column to JSONB
-- removes the string-parsing dance and lets PostgreSQL index the data
-- structurally.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'user_open_to'
          AND column_name = 'value'
          AND data_type IN ('character varying', 'text')
    ) THEN
        -- USING handles legacy rows that look like a single label or
        -- comma-separated list. We coerce to a JSON array of strings so
        -- callers can rely on a stable shape after the migration.
        -- Drop old VARCHAR/TEXT default first; PostgreSQL otherwise tries to
        -- cast it implicitly during TYPE change and fails on some databases.
        ALTER TABLE user_open_to
            ALTER COLUMN value DROP DEFAULT;

        ALTER TABLE user_open_to
            ALTER COLUMN value TYPE JSONB USING (
                CASE
                    WHEN value IS NULL OR value = '' THEN '[]'::jsonb
                    WHEN value LIKE '[%' OR value LIKE '{%' THEN value::jsonb
                    ELSE to_jsonb(string_to_array(value, ','))
                END
            );

        ALTER TABLE user_open_to
            ALTER COLUMN value SET DEFAULT '[]'::jsonb;
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_user_open_to_value
    ON user_open_to USING GIN (value jsonb_path_ops);
