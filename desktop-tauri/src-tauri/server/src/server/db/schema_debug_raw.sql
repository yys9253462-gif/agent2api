ALTER TABLE request_raw ADD COLUMN truncated INTEGER NOT NULL DEFAULT 0;

-- 旧采集器在 128 KiB 附近回退 UTF-8 边界；保留旧读取侧的「可能截断」提示。
UPDATE request_raw SET truncated = 1
WHERE length(CAST(request_body AS BLOB)) BETWEEN 131069 AND 131072
   OR length(CAST(response_body AS BLOB)) BETWEEN 131069 AND 131072;
