CREATE TABLE IF NOT EXISTS migrations (
  version int NOT NULL PRIMARY KEY,
  timestamp timestamp with time zone DEFAULT (CURRENT_TIMESTAMP)
);
