-- Migration: Structured job application questions
-- Plan §3.15 — JA1/JA2: yes_no + multiple_choice + free_text question types.
--
-- The legacy `jobs.question_one` / `question_two` TEXT columns stay in place
-- for backwards compatibility; any row that has a value there gets surfaced
-- as a `free_text` question in the API response.

CREATE TABLE IF NOT EXISTS job_questions (
    id            BIGSERIAL PRIMARY KEY,
    job_id        BIGINT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    position      SMALLINT NOT NULL DEFAULT 0,
    question      TEXT NOT NULL,
    question_type TEXT NOT NULL DEFAULT 'free_text'
        CHECK (question_type IN ('free_text','yes_no','multiple_choice')),
    options       JSONB NOT NULL DEFAULT '[]'::jsonb,
    required      BOOLEAN NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_job_questions_job
    ON job_questions(job_id, position);

COMMENT ON COLUMN job_questions.options IS
    'Only used for multiple_choice. JSON array of strings. Empty for other types.';

-- Candidates answers are captured in a separate table so we can store the
-- chosen option (for multiple_choice / yes_no) alongside a free-form text.
CREATE TABLE IF NOT EXISTS job_application_answers (
    id              BIGSERIAL PRIMARY KEY,
    application_id  BIGINT NOT NULL REFERENCES job_applications(id) ON DELETE CASCADE,
    question_id     BIGINT NOT NULL REFERENCES job_questions(id) ON DELETE CASCADE,
    answer          TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(application_id, question_id)
);

CREATE INDEX IF NOT EXISTS idx_job_app_answers_app
    ON job_application_answers(application_id);

-- Optional CV/resume URL on the application itself (plan §3.15 JA3).
ALTER TABLE job_applications
    ADD COLUMN IF NOT EXISTS resume_url TEXT;
