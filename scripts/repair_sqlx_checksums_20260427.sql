-- Repair sqlx checksum mismatch: "migration X was previously applied but has been modified"
--
-- This happens when a migration .sql file is edited *after* it was already applied;
-- the schema is usually already correct — only the recorded checksum in
-- `_sqlx_migrations` is out of date.
--
-- 1) Confirm which versions are out of sync:  `sqlx migrate info`
-- 2) If 20260427000001 / 20260427000002 show "different checksum", run this
--    against the same database as your DATABASE_URL.
--
-- To refresh the hex values from your tree (if the files change again):
--   sqlx migrate info
--   (Copy "local migration has checksum" for each version listed as mismatch.)
--
-- PostgreSQL: checksum is stored as BYTEA.

UPDATE _sqlx_migrations
SET checksum = decode(
  'c4de61d7b6b4a9e233bb4b10b906b1d75afed1aef62e90858892ac48b775260ccb1a1d10865efade8a8d91c67b1860c5',
  'hex'
)
WHERE version = 20260427000001;

UPDATE _sqlx_migrations
SET checksum = decode(
  'cd6479bafd8e62bb33c4801c121ed9143ca71c130fad1587d672061874d0cd7b158415975aa6a9fa447a0cb5f17ece99',
  'hex'
)
WHERE version = 20260427000002;

-- Example (psql, PowerShell):
--   $env:DATABASE_URL = "postgres://USER:PASS@localhost:5432/DB"
--   psql $env:DATABASE_URL -f scripts/repair_sqlx_checksums_20260427.sql
-- Windows (no psql in PATH):
--   ..\scripts\repair_sqlx_checksums_20260427.ps1
-- then:
--   sqlx migrate run
