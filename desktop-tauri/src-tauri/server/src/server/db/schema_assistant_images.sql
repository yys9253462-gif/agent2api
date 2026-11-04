CREATE TABLE IF NOT EXISTS assistant_images (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES assistant_sessions(id) ON DELETE CASCADE,
    mime TEXT NOT NULL,
    bytes BLOB NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_assistant_images_session ON assistant_images(session_id);
