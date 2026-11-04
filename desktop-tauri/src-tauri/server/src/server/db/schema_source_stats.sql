ALTER TABLE requests ADD COLUMN api_key_id TEXT NOT NULL DEFAULT '';
ALTER TABLE requests ADD COLUMN api_key_name TEXT NOT NULL DEFAULT '';
ALTER TABLE requests ADD COLUMN api_key_source TEXT NOT NULL DEFAULT '';
ALTER TABLE request_daily ADD COLUMN client_stats TEXT NOT NULL DEFAULT '[]';
ALTER TABLE request_daily ADD COLUMN api_key_stats TEXT NOT NULL DEFAULT '[]';
ALTER TABLE request_daily ADD COLUMN source_stats_version INTEGER NOT NULL DEFAULT 0;
CREATE INDEX IF NOT EXISTS idx_requests_client_ts ON requests(client, ts);
CREATE INDEX IF NOT EXISTS idx_requests_api_key_ts ON requests(api_key_source, api_key_id, ts);
