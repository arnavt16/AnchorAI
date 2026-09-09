-- Anchor initial schema (schema_version = 1)
--
-- Single-user local vault. No cloud user IDs, no row-level security —
-- desktop OS file permissions are the access boundary (see SECURITY.md).
-- All timestamps are UTC RFC3339 strings. All IDs are UUID v4 text.

PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS app_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- singleton row
    schema_version INTEGER NOT NULL DEFAULT 1,
    display_name TEXT,
    timezone TEXT,
    local_ai_enabled INTEGER NOT NULL DEFAULT 0 CHECK (local_ai_enabled IN (0, 1)),
    consent_version TEXT,
    consent_at TEXT,
    consent_withdrawn_at TEXT,
    chat_model TEXT,
    chat_model_digest TEXT,
    embedding_model TEXT,
    embedding_model_digest TEXT,
    embedding_dimension INTEGER,
    embedding_space_version INTEGER NOT NULL DEFAULT 1,
    support_country TEXT,
    vault_generation INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS journal_entries (
    id TEXT PRIMARY KEY,
    title TEXT,
    body TEXT NOT NULL,
    mood TEXT,
    tags TEXT NOT NULL DEFAULT '[]', -- JSON array of strings
    origin TEXT NOT NULL DEFAULT 'user' CHECK (origin IN ('user', 'imported', 'saved_reflection')),
    memory_enabled INTEGER NOT NULL DEFAULT 0 CHECK (memory_enabled IN (0, 1)),
    content_version INTEGER NOT NULL DEFAULT 1,
    aggregate_version INTEGER NOT NULL DEFAULT 1,
    indexing_status TEXT NOT NULL DEFAULT 'excluded'
        CHECK (indexing_status IN ('excluded', 'pending', 'processing', 'ready', 'failed')),
    vault_generation INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_entries_created ON journal_entries (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_entries_memory ON journal_entries (memory_enabled, indexing_status);

CREATE TABLE IF NOT EXISTS worries (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    worry_text TEXT NOT NULL,
    expected_outcome TEXT,
    status TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open', 'resolved', 'archived')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (entry_id) -- one worry per entry in the MVP
);

CREATE INDEX IF NOT EXISTS idx_worries_status ON worries (status, created_at DESC);

CREATE TABLE IF NOT EXISTS worry_outcomes (
    id TEXT PRIMARY KEY,
    worry_id TEXT NOT NULL REFERENCES worries(id) ON DELETE CASCADE,
    recorded_at TEXT NOT NULL,
    outcome_text TEXT NOT NULL,
    result_category TEXT CHECK (
        result_category IS NULL OR result_category IN (
            'better_than_expected', 'about_as_expected', 'harder_than_expected', 'mixed', 'still_unsure'
        )
    ),
    reflection TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_outcomes_worry ON worry_outcomes (worry_id, recorded_at);

CREATE TABLE IF NOT EXISTS small_steps (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    worry_id TEXT REFERENCES worries(id) ON DELETE CASCADE,
    action_text TEXT NOT NULL,
    feedback TEXT CHECK (feedback IS NULL OR feedback IN ('helped', 'did_not_help', 'unsure', 'not_tried')),
    feedback_note TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_steps_entry ON small_steps (entry_id);
CREATE INDEX IF NOT EXISTS idx_steps_worry ON small_steps (worry_id);

-- retrieval_chunks: derived data, safe to fully rebuild. source_kind fans
-- out to exactly one of worry_id / outcome_id / small_step_id (or none for
-- source_kind = 'entry'); enforced in application code (see indexing/chunker.rs)
-- since SQLite lacks a native "exactly one of" constraint expression across
-- nullable FKs cleanly — this is intentionally not a bare polymorphic ID.
CREATE TABLE IF NOT EXISTS retrieval_chunks (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    source_kind TEXT NOT NULL CHECK (source_kind IN ('entry', 'worry', 'outcome', 'small_step')),
    worry_id TEXT REFERENCES worries(id) ON DELETE CASCADE,
    outcome_id TEXT REFERENCES worry_outcomes(id) ON DELETE CASCADE,
    small_step_id TEXT REFERENCES small_steps(id) ON DELETE CASCADE,
    chunk_index INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    embedding_blob BLOB NOT NULL,
    embedding_dimension INTEGER NOT NULL,
    embedding_model_digest TEXT NOT NULL,
    embedding_space_version INTEGER NOT NULL,
    source_version INTEGER NOT NULL,
    aggregate_version INTEGER NOT NULL,
    vault_generation INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chunks_entry ON retrieval_chunks (entry_id);
CREATE INDEX IF NOT EXISTS idx_chunks_space ON retrieval_chunks (embedding_space_version, vault_generation);

CREATE TABLE IF NOT EXISTS indexing_jobs (
    id TEXT PRIMARY KEY,
    entry_id TEXT NOT NULL REFERENCES journal_entries(id) ON DELETE CASCADE,
    requested_aggregate_version INTEGER NOT NULL,
    requested_embedding_space_version INTEGER NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'failed')),
    attempt_count INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT,
    safe_error_code TEXT,
    vault_generation INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (entry_id, requested_aggregate_version, requested_embedding_space_version)
);

CREATE INDEX IF NOT EXISTS idx_jobs_status ON indexing_jobs (status, next_attempt_at);

INSERT OR IGNORE INTO app_settings (id, schema_version, created_at, updated_at)
VALUES (1, 1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), strftime('%Y-%m-%dT%H:%M:%fZ', 'now'));
