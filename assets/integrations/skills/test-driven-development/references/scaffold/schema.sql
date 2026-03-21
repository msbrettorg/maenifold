-- pipeline.db cache schema (reference only)
-- This file documents the DB schema used by the gate CLI.
-- To create/rebuild the DB, use: gate sync --rebuild <ROOT>
-- The DB is a rebuildable cache; task files are authoritative.

-- Pipeline DB Schema
-- Files on disk are the artifacts. DB is the index + state machine.

CREATE TABLE requirements (
    id TEXT PRIMARY KEY,
    type TEXT NOT NULL CHECK(type IN ('FR','NFR')),
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE traceability (
    test_case TEXT PRIMARY KEY,
    requirement TEXT NOT NULL REFERENCES requirements(id),
    component TEXT NOT NULL,
    test_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE tasks (
    file TEXT PRIMARY KEY,
    title TEXT,
    requirement TEXT NOT NULL REFERENCES requirements(id),
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK(status IN ('pending','red','blue','done','stuck')),
    location TEXT NOT NULL DEFAULT 'todo'
        CHECK(location IN ('todo','done')),
    attempts INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE work_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file TEXT NOT NULL REFERENCES tasks(file),
    stage TEXT NOT NULL CHECK(stage IN ('swe','red','blue')),
    agent_model TEXT,
    verdict TEXT,
    duration_seconds INTEGER,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_requirement ON tasks(requirement);
CREATE INDEX idx_traceability_requirement ON traceability(requirement);
CREATE INDEX idx_work_log_file ON work_log(file);
