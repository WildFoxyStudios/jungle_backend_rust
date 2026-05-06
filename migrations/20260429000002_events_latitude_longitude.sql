-- Optional pinned coordinates for event venues (Google Maps / embed MapView).
ALTER TABLE events
    ADD COLUMN IF NOT EXISTS latitude DOUBLE PRECISION,
    ADD COLUMN IF NOT EXISTS longitude DOUBLE PRECISION;
