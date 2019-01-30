CREATE TABLE events (
   event_id bigserial NOT NULL PRIMARY KEY,
   entity_type text NOT NULL,
   entity_id text NOT NULL,
   sequence bigint CHECK (sequence > 0) NOT NULL,
   event_type text NOT NULL,
   payload jsonb NOT NULL,
   metadata jsonb NOT NULL,
   timestamp timestamp with time zone DEFAULT (CURRENT_TIMESTAMP),
   UNIQUE (entity_type, entity_id, sequence)
);

CREATE TABLE snapshots (
  snapshot_id bigserial NOT NULL PRIMARY KEY,
  entity_type text NOT NULL,
  entity_id text NOT NULL,
  sequence bigint CHECK (sequence >= 0) NOT NULL,
  payload jsonb NOT NULL,
  UNIQUE (entity_type, entity_id, sequence)
);
