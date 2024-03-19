CREATE TABLE workspace_snapshots
(
    key               text                      NOT NULL PRIMARY KEY,
    sort_key          text                      NOT NULL,
    created_at        timestamp with time zone  NOT NULL DEFAULT CLOCK_TIMESTAMP(),
    value             bytea                     NOT NULL,
    serialization_lib text                      NOT NULL DEFAULT 'postcard'
);

CREATE INDEX IF NOT EXISTS workspace_snapshots_sort_key ON workspace_snapshots (sort_key);
