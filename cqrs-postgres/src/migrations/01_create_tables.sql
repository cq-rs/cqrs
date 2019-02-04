CREATE TABLE events (
   event_id bigserial NOT NULL PRIMARY KEY,
   aggregate_type text NOT NULL,
   entity_id text NOT NULL,
   sequence bigint CHECK (sequence > 0) NOT NULL,
   event_type text NOT NULL,
   payload jsonb NOT NULL,
   metadata jsonb NOT NULL,
   timestamp timestamp with time zone DEFAULT (CURRENT_TIMESTAMP),
   UNIQUE (aggregate_type, entity_id, sequence)
);

CREATE TABLE snapshots (
  snapshot_id bigserial NOT NULL PRIMARY KEY,
  aggregate_type text NOT NULL,
  entity_id text NOT NULL,
  sequence bigint CHECK (sequence >= 0) NOT NULL,
  payload jsonb NOT NULL,
  UNIQUE (aggregate_type, entity_id, sequence)
);

INSERT INTO migrations (version) VALUES (1);
