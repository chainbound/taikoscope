-- Create schema_migrations table to track applied migrations
CREATE TABLE IF NOT EXISTS ${DB}.schema_migrations (
    version UInt32,
    name String,
    applied_at DateTime64(3) DEFAULT now64(),
    checksum String
) ENGINE = MergeTree()
ORDER BY (version);
